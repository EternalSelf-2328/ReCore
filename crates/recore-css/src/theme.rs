//! Reads a Discord CSS theme into the colours and look ReCore paints with.
//!
//! A theme is a set of custom properties on the page's root (`:root`,
//! `html`, `.theme-dark`, `.theme-light`), usually Discord's own tokens
//! (`--background-primary`, `--text-normal`, `--brand-experiment`), plus the
//! theme's own variables. [`load`] follows them through `var()` and
//! `color-mix()`, picks the dark or light set for the client's appearance,
//! and maps them onto ReCore's [`Palette`]. ReCord's `--rl-*` palette is read
//! as well, so ReCord's own CSS theme gives the same colours as the built-in
//! theme.
//!
//! What CSS cannot carry into a native window is styling for Discord's DOM:
//! rules on `.messageContent_...`, `@keyframes`, layouts and so on have no
//! nodes to match here. Themes load their colours, fonts, radius, background
//! picture and the ReCord effects they name; the rest is kept in the file for
//! people, not painted.
//!
//! `@import`s are local by default. [`load_with_imports`] and
//! [`scan_with_imports`] take a fetch callback and also read `http:` and
//! `https:` imports, their nested imports and remote `@font-face` files.
//! A remote picture is returned as [`Image::Url`] for the caller to fetch
//! when it shows the theme.

use crate::{base64, color, css};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub use crate::color::Rgba;
pub use crate::css::{Meta, Stylesheet};

/// Which appearance a theme is read for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// The theme's own root colours decide, or the desktop's appearance.
    #[default]
    Auto,
    Dark,
    Light,
}

/// How much motion the theme asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Full,
    Subtle,
    Off,
}

/// The ReCord CRT effects a theme names, from 0 to 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Effects {
    pub scanlines: f32,
    pub grain: f32,
    pub vignette: f32,
    pub flicker: bool,
    pub beam: bool,
    pub frame: f32,
    pub avatar_filter: Option<String>,
    pub avatar_scanline: f32,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            scanlines: 0.25,
            grain: 0.18,
            vignette: 0.55,
            flicker: false,
            beam: true,
            frame: 0.6,
            avatar_filter: None,
            avatar_scanline: 0.14,
        }
    }
}

/// A picture a theme puts behind the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Image {
    /// The bytes a `data:` URI carries.
    Data(Vec<u8>),
    /// A file beside the theme.
    File(PathBuf),
    /// A picture on the web, which the caller may choose to fetch.
    Url(String),
}

/// A font a theme carries, ready for the window's font system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub bytes: Vec<u8>,
}

/// What a theme asks of the window besides its colours.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub radius: Option<f32>,
    pub motion: Motion,
    pub pixel_icons: bool,
    pub effects: Effects,
    pub background: Option<Image>,
    /// How far the background picture is dimmed, 0..1.
    pub background_dim: Option<f32>,
    /// The accent ReCore's own controls use, as the theme names it.
    pub accent: Option<Rgba>,
}

/// A theme's colours, one field per colour ReCore's window paints with. The
/// names are the ones `recore_core::theme::Theme` uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub dark: bool,
    pub rail: Rgba,
    pub sidebar: Rgba,
    pub panel: Rgba,
    pub chat: Rgba,
    pub stage: Rgba,
    pub raised: Rgba,
    pub field: Rgba,
    pub sunken: Rgba,
    pub hover: Rgba,
    pub active: Rgba,
    pub row_hover: Rgba,
    pub line: Rgba,
    pub edge: Rgba,
    pub outline: Rgba,
    pub outline_chat: Rgba,
    pub outline_field: Rgba,
    pub text: Rgba,
    pub secondary: Rgba,
    pub muted: Rgba,
    pub faint: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub on_accent: Rgba,
    pub accent_soft: Rgba,
    pub danger: Rgba,
    pub danger_hover: Rgba,
    pub on_danger: Rgba,
    pub live: Rgba,
    pub loud: Rgba,
    pub link: Rgba,
    pub button_default: Rgba,
    pub button_default_hover: Rgba,
    pub overlay: Rgba,
    pub shimmer: Rgba,
    pub reply_spine: Rgba,
    pub tooltip: Rgba,
    pub avatar: Rgba,
}

/// A theme read from CSS, with what it says about itself.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeFile {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub source: Option<String>,
    pub path: Option<PathBuf>,
    pub palette: Palette,
    pub look: Look,
    /// Colours and shape values won by selectors that name the native
    /// sidebar, chat, member list and so on.
    pub native: crate::NativeStyle,
    pub fonts: Vec<Font>,
    /// The custom properties Appearance can change, with their labels.
    pub options: Vec<crate::ThemeOption>,
    pub warnings: Vec<String>,
}

impl ThemeFile {
    /// The CSS file it came from, if it came from one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// What [`scan`] found.
#[derive(Clone, Debug, Default)]
pub struct Scan {
    pub themes: Vec<ThemeFile>,
    pub problems: Vec<Problem>,
}

/// A theme file that could not be read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub path: PathBuf,
    pub message: String,
}

/// How to read a theme.
#[derive(Clone, Debug)]
pub struct Options {
    pub mode: Mode,
    /// A colour to force on the theme, as ReCore's accent picker does.
    pub accent: Option<Rgba>,
    /// Custom properties to force over the theme's, for the settings the
    /// window writes (`--rl-scanline-opacity`, `--rl-motion`, and so on).
    pub overrides: Vec<(String, String)>,
    /// Whether to read the fonts the theme carries.
    pub fonts: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            accent: None,
            overrides: Vec::new(),
            fonts: true,
        }
    }
}

impl Options {
    pub fn with_mode(mode: Mode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }
}

/// Reads a theme file and the fonts and pictures beside it.
pub fn load(path: impl AsRef<Path>) -> Result<ThemeFile, String> {
    load_with(path, &Options::default())
}

/// Reads a theme file with `options`, downloading the `http:` and `https:`
/// imports and fonts it names through `fetch`. A remote picture stays an
/// [`Image::Url`], which the caller fetches when it shows the theme.
pub fn load_with_imports(
    path: impl AsRef<Path>,
    options: &Options,
    fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<ThemeFile, String> {
    let path = path.as_ref();
    let css = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let directory = path.parent().map(Path::to_path_buf);
    let mut theme = load_str_with_imports(&css, directory.as_deref(), options, fetch)?;
    theme.path = Some(path.to_path_buf());
    if theme.name == "CSS theme"
        && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
    {
        stem.trim_end_matches(".theme").clone_into(&mut theme.name);
    }
    theme.id = format!("css:{}", slug(theme.name.as_str()));
    Ok(theme)
}

/// Reads a theme file with `options`.
pub fn load_with(path: impl AsRef<Path>, options: &Options) -> Result<ThemeFile, String> {
    load_with_imports(path, options, &mut |_| None)
}

/// Reads the CSS `css`, with `directory` as the base for its imports, fonts
/// and pictures.
pub fn load_str(
    css: &str,
    directory: Option<&Path>,
    options: &Options,
) -> Result<ThemeFile, String> {
    load_str_with_imports(css, directory, options, &mut |_| None)
}

/// [`load_str`], downloading remote imports and fonts through `fetch`.
pub fn load_str_with_imports(
    css: &str,
    directory: Option<&Path>,
    options: &Options,
    fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<ThemeFile, String> {
    let meta = Meta::read(css);
    let mut warnings = Vec::new();
    let sheet = css::parse_in(css, directory, &mut |base, path| {
        if is_remote(path) || base.is_some_and(remote_path) {
            let url = remote_target(base, path);
            match fetch(&url) {
                Some(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => Some((text, PathBuf::from(url))),
                    Err(_) => {
                        warnings.push(format!("the @import {url} is not UTF-8 CSS"));
                        None
                    }
                },
                None => None,
            }
        } else {
            read_import(base, path, &mut warnings)
        }
    });
    let mut fonts = Vec::new();
    if options.fonts {
        for face in &sheet.fonts {
            let mut bytes = face.data.clone();
            if bytes.is_none() {
                if let Some(url) = remote_source(&face.source, face.origin.as_deref()) {
                    bytes = fetch(&url);
                    if bytes.is_none() {
                        warnings.push(format!("could not download the font {url}"));
                    }
                } else if !face.origin.as_deref().is_some_and(is_remote) {
                    let base = face
                        .origin
                        .as_deref()
                        .filter(|origin| !is_remote(origin))
                        .map(Path::new)
                        .or(directory);
                    bytes = read_font(base, &face.source, &mut warnings);
                }
            }
            if let Some(bytes) = bytes
                && !bytes.is_empty()
            {
                fonts.push(Font {
                    family: face.family.clone(),
                    weight: face.weight,
                    italic: face.italic,
                    bytes,
                });
            }
        }
    }
    let collected = collect(&sheet);
    let dark = choose_dark(&collected, options.mode, &mut warnings);
    let mut vars = collected.vars(dark);
    if let Some(accent) = options.accent {
        let hex = accent.hex();
        for name in [
            "--rl-accent",
            "--rl-signal",
            "--brand-experiment",
            "--brand-500",
            "--brand-360",
            "--control-primary-background-default",
        ] {
            vars.insert(&css::Declaration {
                name: name.into(),
                value: hex.clone(),
                important: true,
            });
        }
        let tint = vars
            .first_color(&["--rl-tint"], !dark)
            .unwrap_or(accent.ink())
            .hex();
        vars.insert(&css::Declaration {
            name: "--rl-signal-hot".into(),
            value: format!("color-mix(in srgb, {hex} 80%, {tint})"),
            important: true,
        });
        vars.insert(&css::Declaration {
            name: "--rl-signal-soft".into(),
            value: format!("color-mix(in srgb, {hex} 58%, {tint})"),
            important: true,
        });
    }
    for (name, value) in &options.overrides {
        vars.insert(&css::Declaration {
            name: name.clone(),
            value: value.clone(),
            important: true,
        });
    }
    if vars.contains("--rl-accent") && !vars.contains("--rl-signal") {
        vars.insert(&css::Declaration {
            name: "--rl-signal".into(),
            value: "var(--rl-accent)".into(),
            important: false,
        });
    }
    warnings.extend(sheet.warnings);
    let palette = palette(&vars, dark);
    let native = crate::NativeStyle::compute(&sheet.rules, &vars, dark);
    let mut look = look(&vars, palette, directory, &fonts, options, &mut warnings);
    look.radius = native.radius.or(look.radius);
    look.font_family = native.font_family.clone().or(look.font_family);
    look.font_size = native.font_size.or(look.font_size);
    look.letter_spacing = native.letter_spacing.or(look.letter_spacing);
    let mut specs = crate::options::merge(
        crate::options::detect_with_notes(&vars, dark, &sheet.notes, &sheet.properties),
        crate::options::detect_with_notes(
            &collected.vars(!dark),
            !dark,
            &sheet.notes,
            &sheet.properties,
        ),
        !dark,
    );
    crate::options::apply_meta(&mut specs, &meta.vars);
    let name = meta.name.clone().unwrap_or_else(|| "CSS theme".to_owned());
    Ok(ThemeFile {
        id: format!("css:{}", slug(&name)),
        name,
        author: meta.author,
        description: meta.description,
        version: meta.version,
        license: meta.license,
        source: meta.source.or(meta.website),
        path: None,
        palette,
        look,
        native,
        fonts,
        options: specs,
        warnings,
    })
}

/// Whether `path` was written as an absolute web address.
fn is_remote(path: &str) -> bool {
    path.get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || path
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || path.starts_with("//")
}

fn remote_path(path: &Path) -> bool {
    is_remote(&path.to_string_lossy())
}

/// The web address a remote import names, resolved against the remote file
/// that imports it.
fn remote_target(base: Option<&Path>, path: &str) -> String {
    if path.starts_with("//") {
        return format!("https:{path}");
    }
    if is_remote(path) {
        return path.to_owned();
    }
    let Some(base) = base.filter(|base| remote_path(base)) else {
        return path.to_owned();
    };
    let base = base.to_string_lossy();
    let scheme = base.split("://").next().unwrap_or("https");
    if let Some(path) = path.strip_prefix('/') {
        let host = base
            .split("://")
            .nth(1)
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("");
        return format!("{scheme}://{host}/{path}");
    }
    normalize_remote(&format!("{}/{}", base.trim_end_matches('/'), path))
}

/// Collapses `./` and `../` in a web address, as a server would.
fn normalize_remote(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let Some((host, path)) = rest.split_once('/') else {
        return url.to_owned();
    };
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    if segments.is_empty() {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}/{}", segments.join("/"))
    }
}

/// The first `url(...)` in a `src` or `background` value when it is a web
/// address, resolved against a remote stylesheet's own folder.
fn remote_source(source: &str, origin: Option<&str>) -> Option<String> {
    let start = css::find_ignore_case(source, "url(")?;
    let rest = &source[start + 4..];
    let end = rest.find(')')?;
    let path = rest[..end].trim().trim_matches(['"', '\'']);
    if path.starts_with("data:") {
        return None;
    }
    if is_remote(path) {
        return Some(remote_target(None, path));
    }
    if origin.is_some_and(is_remote) && !path.starts_with("file://") {
        return Some(remote_target(origin.map(Path::new), path));
    }
    None
}

fn read_import(
    directory: Option<&Path>,
    path: &str,
    warnings: &mut Vec<String>,
) -> Option<(String, PathBuf)> {
    let path = path.trim().trim_start_matches("file://");
    let full = resolve_path(directory, path)?;
    match fs::read_to_string(&full) {
        Ok(text) => Some((text, full)),
        Err(error) => {
            warnings.push(format!("could not read {}: {error}", full.display()));
            None
        }
    }
}

fn resolve_path(directory: Option<&Path>, path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    Some(directory?.join(path))
}

/// Reads a font named by a `src: url(...)`.
fn read_font(
    directory: Option<&Path>,
    source: &str,
    warnings: &mut Vec<String>,
) -> Option<Vec<u8>> {
    let start = css::find_ignore_case(source, "url(")?;
    let rest = &source[start + 4..];
    let end = rest.find(')')?;
    let path = rest[..end].trim().trim_matches(['"', '\'']);
    if is_remote(path) {
        // The caller downloads remote fonts; a missing fetcher leaves none.
        return None;
    }
    let path = path.trim_start_matches("file://");
    let full = resolve_path(directory, path)?;
    match fs::read(&full) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            warnings.push(format!(
                "could not read the font {}: {error}",
                full.display()
            ));
            None
        }
    }
}

/// Every declaration that writes on the root, with the appearance it applies
/// to, in source order: `None` for both, `Some(true)` for the dark palette and
/// `Some(false)` for the light one.
#[derive(Default)]
struct Collected {
    entries: Vec<Entry>,
}

struct Entry {
    declaration: css::Declaration,
    dark: Option<bool>,
    specificity: u32,
    order: usize,
    origin: Option<String>,
    layer: Option<u32>,
}

fn collect(sheet: &Stylesheet) -> Collected {
    let mut collected = Collected::default();
    let mut order = 0usize;
    for rule in &sheet.rules {
        for selector in &rule.selectors {
            let Some((selector_dark, specificity)) =
                css::root_selector_with_specificity(selector, true)
            else {
                continue;
            };
            let explicit = selector_is_explicit(selector);
            let scope = match (explicit.then_some(selector_dark), rule.color_scheme) {
                // `.theme-light` inside `prefers-color-scheme: dark` can never
                // apply, and neither can the other way around.
                (Some(a), Some(b)) if a != b => continue,
                (Some(dark), _) => Some(dark),
                (None, Some(dark)) => Some(dark),
                (None, None) => None,
            };
            for declaration in &rule.declarations {
                collected.entries.push(Entry {
                    declaration: declaration.clone(),
                    dark: scope,
                    specificity,
                    order,
                    origin: rule.origin.clone(),
                    layer: rule.layer,
                });
                order += 1;
            }
        }
    }
    collected
}

/// Whether the selector names an appearance itself (`.theme-dark`,
/// `.theme-light`), rather than leaving it to the client.
fn selector_is_explicit(selector: &str) -> bool {
    let lower = selector.to_ascii_lowercase();
    lower.contains("theme-light")
        || lower.contains("theme-dark")
        || lower.contains("theme-darker")
        || lower.contains("theme-midnight")
        || lower.contains("theme-ash")
        || lower.contains("data-rl-mode=\"light\"")
        || lower.contains("data-rl-mode='light'")
        || lower.contains("data-rl-mode=\"dark\"")
        || lower.contains("data-rl-mode='dark'")
}

impl Entry {
    /// CSS layers outrank specificity: a later layer beats an earlier one,
    /// and unlayered rules beat every layer. The weight packs the layer in
    /// above the selector's specificity.
    fn layer_specificity(&self) -> u32 {
        let layer = self.layer.map_or(1_000_000_000, |rank| {
            rank.saturating_add(1).saturating_mul(1_000_000)
        });
        layer.saturating_add(self.specificity.min(999_999))
    }
}

impl Collected {
    /// The cascade for `dark`.
    fn vars_with(&self, dark: bool) -> css::Vars {
        let mut vars = css::Vars::default();
        for entry in &self.entries {
            if entry.dark.is_some_and(|scope| scope != dark) {
                continue;
            }
            vars.insert_weighted(
                &entry.declaration,
                entry.layer_specificity(),
                entry.order,
                entry.origin.as_deref(),
            );
        }
        vars
    }

    fn vars(&self, dark: bool) -> css::Vars {
        self.vars_with(dark)
    }

    /// The appearance the theme asks for: an explicit `color-scheme`, then
    /// the ground it paints. The base declarations are read as light first,
    /// then the dark set, then the light set.
    fn appearance(&self) -> Option<bool> {
        let mut vars = css::Vars::default();
        for entry in &self.entries {
            if entry.dark.is_none() {
                vars.insert_weighted(
                    &entry.declaration,
                    entry.layer_specificity(),
                    entry.order,
                    entry.origin.as_deref(),
                );
            }
        }
        if let Some(dark) = appearance_of(&vars, true) {
            return Some(dark);
        }
        let dark_vars = self.vars_with(true);
        if let Some(dark) = appearance_of(&dark_vars, false) {
            return Some(dark);
        }
        let light_vars = self.vars_with(false);
        appearance_of(&light_vars, true)
    }
}

/// The appearance a set of root declarations names: `color-scheme`, then the
/// brightness of the ground. `light` chooses `light-dark()`'s branch.
fn appearance_of(vars: &css::Vars, light: bool) -> Option<bool> {
    if let Some(scheme) = vars.get("color-scheme") {
        let lower = scheme.to_ascii_lowercase();
        for word in lower.split_whitespace() {
            if word == "dark" {
                return Some(true);
            }
            if word == "light" {
                return Some(false);
            }
        }
    }
    let background = vars.first_color(
        &[
            "--background-primary",
            "--background-base-lower",
            "--chat-background",
            "--background",
            "--background-secondary",
            "--backgroundsecondary",
            "--backgroundprimary",
        ],
        light,
    )?;
    Some(background.is_dark())
}

fn choose_dark(collected: &Collected, mode: Mode, warnings: &mut Vec<String>) -> bool {
    match mode {
        Mode::Dark => true,
        Mode::Light => false,
        Mode::Auto => {
            if let Some(dark) = collected.appearance() {
                return dark;
            }
            let has_light = collected
                .entries
                .iter()
                .any(|entry| entry.dark == Some(false));
            if has_light {
                return false;
            }
            warnings.push(
                "the theme names no background; read as a dark theme. Choose Light in Appearance if it is a light one."
                    .into(),
            );
            true
        }
    }
}

/// Reads a colour in `names`, with the fallback chain `then` and finally
/// `fallback`.
fn pick(vars: &css::Vars, names: &[&str], light: bool, then: &[&str], fallback: Rgba) -> Rgba {
    vars.first_color(names, light)
        .or_else(|| vars.first_color(then, light))
        .unwrap_or(fallback)
}

/// A translucent token laid over `ground`, as the browser paints it.
fn over(vars: &css::Vars, names: &[&str], light: bool, ground: Rgba) -> Option<Rgba> {
    vars.first_color(names, light)
        .map(|color| color.over(ground))
}

/// A px, rem or em length in a custom property.
fn length(vars: &css::Vars, names: &[&str]) -> Option<f32> {
    for name in names {
        let Some(raw) = vars.get(name) else {
            continue;
        };
        let Some(value) = vars.resolve(raw) else {
            continue;
        };
        let value = value.trim().to_ascii_lowercase();
        let value = if let Some(number) = value.strip_suffix("rem") {
            number.trim().parse::<f32>().ok().map(|value| value * 16.0)
        } else if let Some(number) = value.strip_suffix("em") {
            number.trim().parse::<f32>().ok().map(|value| value * 16.0)
        } else if let Some(number) = value.strip_suffix("px") {
            number.trim().parse::<f32>().ok()
        } else {
            value.parse::<f32>().ok()
        };
        if let Some(value) = value {
            return Some(value);
        }
    }
    None
}

/// A percentage custom property as 0..1 (`25`, `25%` or `0.25`).
fn fraction(vars: &css::Vars, names: &[&str]) -> Option<f32> {
    for name in names {
        let Some(raw) = vars.get(name) else {
            continue;
        };
        let Some(value) = vars.resolve(raw) else {
            continue;
        };
        let value = value.trim();
        if let Some(percent) = value.strip_suffix('%') {
            if let Ok(value) = percent.trim().parse::<f32>() {
                return Some((value / 100.0).clamp(0.0, 1.0));
            }
        } else if let Ok(value) = value.parse::<f32>() {
            // ReCord writes 0.25; older themes write 25 for the same level.
            return Some(if value > 1.0 {
                (value / 100.0).clamp(0.0, 1.0)
            } else {
                value.clamp(0.0, 1.0)
            });
        }
    }
    None
}

/// A CSS duration in seconds (`6s`, `250ms` or `0`).
fn duration(vars: &css::Vars, name: &str) -> Option<f32> {
    let raw = vars.get(name)?;
    let value = vars.resolve(raw)?;
    let value = value.trim().to_ascii_lowercase();
    if let Some(number) = value.strip_suffix("ms") {
        number
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 1000.0)
    } else if let Some(number) = value.strip_suffix('s') {
        number.trim().parse::<f32>().ok()
    } else {
        value.parse::<f32>().ok()
    }
}

fn image(vars: &css::Vars, directory: Option<&Path>, names: &[&str]) -> Option<Image> {
    for name in names {
        let Some(raw) = vars.get(name) else {
            continue;
        };
        let Some(value) = vars.resolve(raw) else {
            continue;
        };
        if value.trim().eq_ignore_ascii_case("none") {
            continue;
        }
        let path = if let Some(start) = css::find_ignore_case(&value, "url(") {
            let rest = &value[start + 4..];
            let Some(end) = rest.find(')') else {
                continue;
            };
            rest[..end].trim().trim_matches(['"', '\''])
        } else {
            // A field the window wrote may hold the path without `url()`.
            let path = value.trim().trim_matches(['"', '\'']);
            if path.is_empty() {
                continue;
            }
            path
        };
        let origin = vars.origin(name);
        let path = path.trim().trim_start_matches("file://");
        if path.starts_with("data:") {
            if let Some(bytes) = css_data_uri(path) {
                return Some(Image::Data(bytes));
            }
        } else if is_remote(path) {
            return Some(Image::Url(remote_target(None, path)));
        } else if origin.is_some_and(is_remote) {
            // A relative picture in a downloaded stylesheet is downloaded
            // from beside it, not looked for on this machine.
            return Some(Image::Url(remote_target(origin.map(Path::new), path)));
        } else {
            let base = origin.filter(|origin| !is_remote(origin)).map(Path::new);
            if let Some(full) = resolve_path(base.or(directory), path)
                && full.exists()
            {
                return Some(Image::File(full));
            }
        }
    }
    None
}

fn css_data_uri(text: &str) -> Option<Vec<u8>> {
    let inside = text.strip_prefix("data:")?;
    let (meta, data) = inside.split_once(',')?;
    if meta.contains(";base64") {
        base64::decode(data)
    } else {
        Some(percent_decode(data))
    }
}

fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            decoded.push(((high << 4) | low) as u8);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    decoded
}

/// Maps a theme's custom properties onto ReCore's colours.
pub fn palette(vars: &css::Vars, dark: bool) -> Palette {
    let light = !dark;
    // The modern Discord tokens, then the ones before Discord's 2024 redesign.
    let ground = pick(
        vars,
        &[
            "--background-primary",
            "--backgroundprimary",
            "--background-base-lower",
            "--chat-background",
            "--background",
        ],
        light,
        &[
            "--background-secondary",
            "--backgroundsecondary",
            "--background-base-lowest",
            "--background-tertiary",
        ],
        if dark {
            Rgba::rgb(0x31, 0x33, 0x38)
        } else {
            Rgba::rgb(0xff, 0xff, 0xff)
        },
    );
    let text = pick(
        vars,
        &[
            "--text-normal",
            "--text-default",
            "--header-primary",
            "--textbrightest",
        ],
        light,
        &["--primary-100", "--text-strong", "--textbright", "--white"],
        ground.ink(),
    );
    let accent = pick(
        vars,
        &[
            "--brand-experiment",
            "--brand-500",
            "--control-primary-background-default",
            "--background-brand",
            "--blurple-50",
            "--accentcolor",
        ],
        light,
        &["--brand-360", "--text-brand", "--accentcolor2"],
        Rgba::rgb(0x58, 0x65, 0xf2),
    );
    let wash = if dark {
        Rgba::rgb(255, 255, 255)
    } else {
        Rgba::rgb(0, 0, 0)
    };
    let raised = pick(
        vars,
        &[
            "--background-floating",
            "--backgroundfloating",
            "--background-surface-high",
            "--card-background-default",
            "--background-nested-floating",
        ],
        light,
        &[
            "--background-secondary-alt",
            "--background-secondary",
            "--backgroundsecondary",
        ],
        ground.mix(wash, 0.06),
    );
    let field = pick(
        vars,
        &[
            "--channeltextarea-background",
            "--input-background-default",
            "--backgroundcode",
            "--background-code",
        ],
        light,
        &[
            "--background-secondary-alt",
            "--backgroundsecondaryalt",
            "--background-tertiary",
            "--backgroundtertiary",
        ],
        ground.mix(if dark { Rgba::rgb(0, 0, 0) } else { wash }, 0.05),
    );
    let sunken = pick(
        vars,
        &[
            "--background-base-lowest",
            "--background-tertiary",
            "--backgroundtertiary",
            "--background-accent",
        ],
        light,
        &["--background-secondary-alt", "--backgroundsecondaryalt"],
        ground.mix(if dark { Rgba::rgb(0, 0, 0) } else { wash }, 0.08),
    );
    let sidebar = pick(
        vars,
        &[
            "--background-secondary",
            "--backgroundsecondary",
            "--background-base-low",
            "--panel-bg",
        ],
        light,
        &["--background-secondary-alt", "--backgroundsecondaryalt"],
        ground.mix(wash, 0.03),
    );
    let line = pick(
        vars,
        &[
            "--border-subtle",
            "--border-normal",
            "--background-modifier-accent",
            "--app-frame-border",
        ],
        light,
        &["--border", "--background-modifier-strong"],
        text.mix(ground, 0.76),
    );
    let hover = over(
        vars,
        &[
            "--background-modifier-hover",
            "--interactive-background-hover",
            "--message-background-hover",
            "--background-modifier-selected",
        ],
        light,
        ground,
    )
    .unwrap_or_else(|| ground.mix(accent, 0.06));
    let active = over(
        vars,
        &[
            "--background-modifier-active",
            "--interactive-background-active",
            "--background-modifier-selected",
        ],
        light,
        ground,
    )
    .unwrap_or_else(|| ground.mix(accent, 0.12));
    let row_hover = over(
        vars,
        &["--message-background-hover", "--background-modifier-hover"],
        light,
        ground,
    )
    .unwrap_or_else(|| ground.mix(accent, 0.045));
    let danger = pick(
        vars,
        &[
            "--status-danger",
            "--control-critical-primary-background-default",
            "--red-400",
            "--danger",
            "--dangercolor",
        ],
        light,
        &["--red-500"],
        Rgba::rgb(0xf2, 0x3f, 0x43),
    );
    let on_accent = vars
        .first_color(
            &["--control-primary-text-default", "--text-invert", "--white"],
            light,
        )
        .unwrap_or_else(|| accent.ink());
    let secondary = pick(
        vars,
        &["--header-secondary", "--text-muted", "--interactive-normal"],
        light,
        &["--text-subtle"],
        text.mix(ground, 0.35),
    );
    let muted = pick(
        vars,
        &["--text-muted", "--interactive-normal", "--channels-default"],
        light,
        &["--interactive-muted"],
        text.mix(ground, 0.52),
    );
    let faint = pick(
        vars,
        &["--text-subtle", "--interactive-muted", "--channel-icon"],
        light,
        &[],
        text.mix(ground, 0.65),
    );
    let live = pick(
        vars,
        &[
            "--status-positive",
            "--green-360",
            "--online",
            "--successcolor",
        ],
        light,
        &["--status-green"],
        Rgba::rgb(0x23, 0xa5, 0x5a),
    );
    let loud = pick(
        vars,
        &[
            "--status-warning",
            "--yellow-300",
            "--idle",
            "--warningcolor",
        ],
        light,
        &["--status-yellow"],
        Rgba::rgb(0xf0, 0xb2, 0x32),
    );
    let link = pick(
        vars,
        &["--text-link", "--link", "--brand-360", "--linkcolor"],
        light,
        &["--text-brand"],
        accent,
    );
    let button = pick(
        vars,
        &[
            "--control-secondary-background-default",
            "--button-secondary-background",
            "--background-modifier-selected",
        ],
        light,
        &["--background-secondary-alt"],
        raised,
    );
    let button_hover = pick(
        vars,
        &[
            "--control-secondary-background-hover",
            "--button-secondary-background-hover",
        ],
        light,
        &[],
        button.mix(wash, 0.08),
    );
    let danger_hover = pick(
        vars,
        &["--control-critical-primary-background-hover", "--red-500"],
        light,
        &[],
        danger.mix(wash, 0.12),
    );
    let on_danger = vars
        .first_color(&["--control-critical-primary-text-default"], light)
        .unwrap_or_else(|| danger.ink());
    let tooltip = pick(
        vars,
        &["--background-tooltip", "--background-floating"],
        light,
        &[],
        raised,
    );
    let overlay = if dark {
        Rgba::rgb(255, 255, 255)
    } else {
        Rgba::rgb(0, 0, 0)
    };

    // ReCord's own palette, when the file is ReCord: it has a more precise
    // mapping for every part than the generic Discord tokens above.
    if vars.contains("--rl-ink") {
        return record_palette(vars, dark);
    }

    Palette {
        dark,
        rail: pick(
            vars,
            &[
                "--background-base-lowest",
                "--background-tertiary",
                "--app-frame-background",
            ],
            light,
            &["--background-secondary-alt"],
            sunken,
        ),
        sidebar,
        panel: sidebar,
        chat: ground,
        stage: ground,
        raised,
        field,
        sunken,
        hover,
        active,
        row_hover,
        line,
        edge: pick(
            vars,
            &["--app-frame-border", "--border-strong"],
            light,
            &[],
            line,
        ),
        outline: line,
        outline_chat: line,
        outline_field: line,
        text,
        secondary,
        muted,
        faint,
        accent,
        accent_hover: pick(
            vars,
            &[
                "--brand-560",
                "--control-primary-background-hover",
                "--brand-530",
            ],
            light,
            &[],
            accent.mix(wash, if dark { 0.12 } else { 0.18 }),
        ),
        on_accent,
        accent_soft: accent.with_alpha(0x26),
        danger,
        danger_hover,
        on_danger,
        live,
        loud,
        link,
        button_default: button,
        button_default_hover: button_hover,
        overlay,
        shimmer: if dark {
            Rgba::rgba(255, 255, 255, 0x10)
        } else {
            Rgba::rgba(255, 255, 255, 0x80)
        },
        reply_spine: pick(vars, &["--spine-default"], light, &[], line),
        tooltip,
        avatar: pick(
            vars,
            &["--avatar-background", "--background-modifier-accent"],
            light,
            &[],
            muted,
        ),
    }
}

/// ReCord's `--rl-*` tokens, which name every part it paints.
fn record_palette(vars: &css::Vars, dark: bool) -> Palette {
    let light = !dark;
    let color = |name: &str, fallback: Rgba| vars.first_color(&[name], light).unwrap_or(fallback);
    let ink = color(
        "--rl-ink",
        if dark {
            Rgba::rgb(0x08, 0x09, 0x07)
        } else {
            Rgba::rgb(0xf3, 0xe9, 0xd7)
        },
    );
    let panel = color(
        "--rl-panel",
        if dark {
            Rgba::rgb(0x0e, 0x10, 0x0c)
        } else {
            Rgba::rgb(0xe4, 0xd6, 0xbd)
        },
    );
    let void = color(
        "--rl-void",
        if dark {
            Rgba::rgb(0x05, 0x06, 0x05)
        } else {
            Rgba::rgb(0xef, 0xe4, 0xd0)
        },
    );
    let raised = color(
        "--rl-raised",
        if dark {
            Rgba::rgb(0x13, 0x15, 0x11)
        } else {
            Rgba::rgb(0xdc, 0xcd, 0xb2)
        },
    );
    let text = color(
        "--rl-bone",
        if dark {
            Rgba::rgb(0xe4, 0xcd, 0xb1)
        } else {
            Rgba::rgb(0x2a, 0x1d, 0x14)
        },
    );
    let high = color("--rl-high", raised.mix(text, 0.12));
    let line = color(
        "--rl-line",
        if dark {
            Rgba::rgb(0x2c, 0x24, 0x1d)
        } else {
            Rgba::rgb(0xb7, 0xa3, 0x83)
        },
    );
    let secondary = color("--rl-smoke", text.mix(ink, 0.38));
    let muted = color("--rl-ash", text.mix(ink, 0.5));
    let faint = color("--rl-faint", text.mix(ink, 0.62));
    let accent = color("--rl-signal", Rgba::rgb(0xe9, 0x42, 0x21));
    let accent_hover = color("--rl-signal-hot", accent.mix(color("--rl-tint", text), 0.2));
    let danger = color("--rl-red", Rgba::rgb(0xff, 0x46, 0x26));
    let live = color("--rl-green", Rgba::rgb(0x79, 0xb5, 0x9a));
    let loud = color("--rl-amber", Rgba::rgb(0xdd, 0xa0, 0x52));
    let hover = over(
        vars,
        &[
            "--interactive-background-hover",
            "--background-modifier-hover",
        ],
        light,
        ink,
    )
    .unwrap_or_else(|| ink.mix(accent, 0.065));
    let active = over(
        vars,
        &[
            "--interactive-background-active",
            "--background-modifier-active",
        ],
        light,
        ink,
    )
    .unwrap_or_else(|| ink.mix(accent, 0.12));
    let row_hover = over(
        vars,
        &["--message-background-hover", "--background-modifier-hover"],
        light,
        ink,
    )
    .unwrap_or_else(|| ink.mix(accent, 0.045));
    Palette {
        dark,
        rail: ink,
        sidebar: panel,
        panel,
        chat: ink,
        stage: ink,
        raised,
        field: void,
        sunken: void,
        hover,
        active,
        row_hover,
        line,
        edge: void,
        outline: line,
        outline_chat: line,
        outline_field: line,
        text,
        secondary,
        muted,
        faint,
        accent,
        accent_hover,
        on_accent: void,
        accent_soft: accent.with_alpha(0x29),
        danger,
        danger_hover: color(
            "--control-critical-primary-background-hover",
            danger.mix(
                if dark {
                    Rgba::rgb(255, 255, 255)
                } else {
                    Rgba::rgb(0, 0, 0)
                },
                0.12,
            ),
        ),
        on_danger: void,
        live,
        loud,
        link: color(
            "--rl-signal-soft",
            accent.mix(color("--rl-tint", text), 0.42),
        ),
        button_default: raised,
        button_default_hover: high,
        overlay: if dark {
            Rgba::rgb(255, 255, 255)
        } else {
            Rgba::rgb(0, 0, 0)
        },
        shimmer: if dark {
            Rgba::rgba(255, 255, 255, 0x10)
        } else {
            Rgba::rgba(255, 255, 255, 0x80)
        },
        reply_spine: line,
        tooltip: high,
        avatar: muted,
    }
}

fn look(
    vars: &css::Vars,
    palette: Palette,
    directory: Option<&Path>,
    fonts: &[Font],
    options: &Options,
    warnings: &mut Vec<String>,
) -> Look {
    let font_family = vars
        .get("--font-primary")
        .or_else(|| vars.get("--rl-font"))
        .or_else(|| vars.get("--font"))
        .and_then(|value| vars.resolve(value))
        .and_then(|value| first_family(&value));
    if let Some(family) = &font_family
        && options.fonts
        && !fonts
            .iter()
            .any(|font| font.family.eq_ignore_ascii_case(family))
        && !system_family(family)
    {
        warnings.push(format!(
            "the theme names the font {family}, which it does not carry; the system one is used if it has it"
        ));
    }
    let motion = match vars
        .resolve(vars.get("--rl-motion").unwrap_or("full"))
        .unwrap_or_else(|| "full".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => Motion::Off,
        "subtle" => Motion::Subtle,
        _ => Motion::Full,
    };
    let mut effects = Effects {
        scanlines: fraction(vars, &["--rl-scanline-opacity"]).unwrap_or(0.0),
        grain: fraction(vars, &["--rl-grain-opacity"]).unwrap_or(0.0),
        vignette: fraction(vars, &["--rl-vignette-opacity"]).unwrap_or(0.0),
        flicker: duration(vars, "--rl-flicker-duration").is_some_and(|seconds| seconds > 0.0),
        beam: fraction(vars, &["--rl-beam-opacity"]).is_some_and(|value| value > 0.0),
        frame: fraction(vars, &["--rl-frame-opacity"]).unwrap_or(0.0),
        avatar_filter: vars
            .resolve(vars.get("--rl-avatar-filter").unwrap_or("none"))
            .filter(|value| !value.eq_ignore_ascii_case("none")),
        avatar_scanline: fraction(vars, &["--rl-avatar-scanline"]).unwrap_or(0.0),
    };
    // ReCord's low power drops the moving and static noise; scanlines and the
    // vignette stay, as they do in the theme.
    if vars
        .resolve(vars.get("--rl-lite").unwrap_or("off"))
        .is_some_and(|value| value.eq_ignore_ascii_case("on"))
    {
        effects.grain = 0.0;
        effects.beam = false;
        effects.flicker = false;
    }
    Look {
        font_family,
        font_size: length(vars, &["--font-size", "--text-size"]),
        letter_spacing: length(vars, &["--rl-tracking", "--letter-spacing"]),
        radius: length(vars, &["--rl-radius", "--radius-md", "--radius-sm"]),
        motion,
        pixel_icons: vars
            .resolve(vars.get("--rl-icons").unwrap_or("off"))
            .is_some_and(|value| value.eq_ignore_ascii_case("on")),
        effects,
        background: image(
            vars,
            directory,
            &[
                "--rl-background-image",
                "--background-image",
                "--recore-background",
            ],
        ),
        background_dim: fraction(vars, &["--rl-background-dim"]),
        accent: Some(palette.accent),
    }
}

/// The first concrete family in a CSS `font-family` list.
fn first_family(value: &str) -> Option<String> {
    for family in color::split_top(value, ',') {
        let family = family.trim().trim_matches(['"', '\'']).trim();
        if family.is_empty() {
            continue;
        }
        let lower = family.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "sans-serif"
                | "serif"
                | "monospace"
                | "cursive"
                | "fantasy"
                | "system-ui"
                | "ui-sans-serif"
                | "ui-monospace"
                | "ui-serif"
                | "ui-rounded"
                | "emoji"
                | "math"
                | "fangsong"
                | "inherit"
                | "initial"
                | "unset"
        ) {
            continue;
        }
        return Some(family.to_owned());
    }
    None
}

fn system_family(family: &str) -> bool {
    matches!(
        family.to_ascii_lowercase().as_str(),
        "courier new"
            | "consolas"
            | "monospace"
            | "system-ui"
            | "roboto"
            | "roboto condensed"
            | "noto sans"
            | "inter"
            | "arial"
            | "helvetica"
            | "segoe ui"
            | "ubuntu"
            | "cantarell"
            | "dejavu sans"
    )
}

/// A cheap fingerprint of the theme files in `directories`: their paths,
/// sizes and modification times. The window uses it to reload when a theme
/// file is saved outside ReCore.
pub fn theme_files_fingerprint(directories: &[PathBuf]) -> u64 {
    let mut files = Vec::new();
    for directory in directories {
        let _ = collect_css_files(directory, 0, &mut files);
    }
    files.sort();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for path in files {
        let canonical = fs::canonicalize(&path).unwrap_or(path);
        let meta = fs::metadata(&canonical).ok();
        let len = meta.as_ref().map_or(0, fs::Metadata::len);
        let modified = meta
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |time| time.as_nanos());
        for byte in canonical.to_string_lossy().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= len;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        hash ^= modified as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Reads every `.css` theme in `directories`, in name order, skipping the ones
/// already read and the ones that fail (kept in [`Scan::problems`]).
pub fn scan(directories: &[PathBuf]) -> Scan {
    scan_with_imports(directories, &mut |_| None)
}

/// Finds `.css` files in the folder and in its own subfolders, which theme
/// packs often use. Hidden folders, dependency trees and build caches are
/// left alone.
fn collect_css_files(
    directory: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if depth > 4 {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            if matches!(
                name.as_ref(),
                "node_modules" | "target" | ".git" | ".cache" | "vendor"
            ) {
                continue;
            }
            // A folder inside a theme pack may not be readable; keep the
            // files already found rather than failing the whole folder.
            let _ = collect_css_files(&path, depth + 1, files);
        } else if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("css"))
        {
            files.push(path);
        }
    }
    Ok(())
}

/// A cheap content fingerprint, so the same theme copied into Vesktop's and
/// ReCore's folders appears once.
fn theme_fingerprint(path: &Path) -> Option<u64> {
    let bytes = fs::read(path).ok()?;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in &bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(hash ^ (bytes.len() as u64))
}

/// [`scan`], downloading each theme's remote imports and fonts through
/// `fetch`.
pub fn scan_with_imports(
    directories: &[PathBuf],
    fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Scan {
    let mut scan = Scan::default();
    let mut seen = BTreeSet::new();
    let mut seen_content = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for directory in directories {
        if !directory.exists() {
            // Most people don't have every client installed; a folder that
            // isn't there is not a theme that failed to read.
            continue;
        }
        let mut files = Vec::new();
        if let Err(error) = collect_css_files(directory, 0, &mut files) {
            scan.problems.push(Problem {
                path: directory.clone(),
                message: format!("could not read the themes folder: {error}"),
            });
            continue;
        }
        files.sort();
        for path in files {
            let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if !seen.insert(canonical.clone()) {
                continue;
            }
            if let Some(fingerprint) = theme_fingerprint(&path)
                && !seen_content.insert(fingerprint)
            {
                continue;
            }
            match load_with_imports(&path, &Options::default(), fetch) {
                Ok(mut theme) => {
                    let base = theme.id.clone();
                    let mut number = 2;
                    while !ids.insert(theme.id.clone()) {
                        theme.id = format!("{base}-{number}");
                        number += 1;
                    }
                    scan.themes.push(theme);
                }
                Err(message) => scan.problems.push(Problem { path, message }),
            }
        }
    }
    scan
}

/// The folders Vencord, Vesktop, Equicord, BetterDiscord, Legcord and ReCord
/// keep local themes in, when this desktop has them.
pub fn theme_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Some(config) = config_directory() {
        for name in [
            "Vencord",
            "vesktop",
            "Equicord",
            "Equibop",
            "BetterDiscord",
            "Legcord",
            "Dorion",
            "GoofCord",
            "replugged",
        ] {
            folders.push(config.join(name).join("themes"));
        }
    }
    folders
}

fn config_directory() -> Option<PathBuf> {
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config));
    }
    if cfg!(windows)
        && let Some(appdata) = std::env::var_os("APPDATA")
    {
        return Some(PathBuf::from(appdata));
    }
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support"),
        );
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            dash = false;
        } else if !dash && !slug.is_empty() {
            slug.push('-');
            dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "theme".into()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECORD: &str = r#"
/** @name ReCord @author leriart @description Terminal */
:root {
    --rl-accent: #e94221;
    --rl-signal: var(--rl-accent);
    --rl-signal-hot: color-mix(in srgb, var(--rl-accent) 80%, #fff3d6);
    --rl-signal-soft: color-mix(in srgb, var(--rl-accent) 58%, #fff3d6);
    --rl-void: #050605;
    --rl-ink: #080907;
    --rl-panel: #0e100c;
    --rl-raised: #131511;
    --rl-high: #191c16;
    --rl-line: #2c241d;
    --rl-bone: #e4cdb1;
    --rl-smoke: #a58979;
    --rl-ash: #8c7b6c;
    --rl-faint: #6f5d4f;
    --rl-green: #79b59a;
    --rl-amber: #dda052;
    --rl-red: #ff4626;
    --rl-motion: subtle;
    --rl-icons: on;
    --background-primary: var(--rl-ink);
    --background-secondary: var(--rl-panel);
    --text-normal: var(--rl-bone);
    --font-primary: "Pixel Operator", monospace;
}
html.theme-light:not([data-rl-mode="dark"]) {
    --rl-void: #efe4d0;
    --rl-ink: #f3e9d7;
    --rl-panel: #e4d6bd;
    --rl-raised: #dccdb2;
    --rl-line: #b7a383;
    --rl-bone: #2a1d14;
    --rl-smoke: #57422f;
    --rl-ash: #6b5644;
    --rl-faint: #8a7560;
}
"#;

    #[test]
    fn record_css_gives_record_colours() {
        let theme = load_str(RECORD, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(theme.name, "ReCord");
        assert_eq!(theme.author.as_deref(), Some("leriart"));
        assert_eq!(theme.palette.rail, Rgba::rgb(0x08, 0x09, 0x07));
        assert_eq!(theme.palette.sidebar, Rgba::rgb(0x0e, 0x10, 0x0c));
        assert_eq!(theme.palette.chat, Rgba::rgb(0x08, 0x09, 0x07));
        assert_eq!(theme.palette.accent, Rgba::rgb(0xe9, 0x42, 0x21));
        assert_eq!(theme.palette.accent_hover, Rgba::rgb(0xed, 0x65, 0x45));
        assert_eq!(theme.palette.text, Rgba::rgb(0xe4, 0xcd, 0xb1));
        assert_eq!(theme.look.font_family.as_deref(), Some("Pixel Operator"));
        assert_eq!(theme.look.motion, Motion::Subtle);
        assert!(theme.look.pixel_icons);
        assert!(theme.palette.dark);
    }

    #[test]
    fn the_light_selector_gives_the_paper_palette() {
        let theme = load_str(RECORD, None, &Options::with_mode(Mode::Light)).unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0xf3, 0xe9, 0xd7));
        assert_eq!(theme.palette.text, Rgba::rgb(0x2a, 0x1d, 0x14));
        assert!(!theme.palette.dark);
    }

    #[test]
    fn a_common_discord_theme_maps_its_tokens() {
        let css = r#"
        :root {
            --background-primary: #313338;
            --background-secondary: #2b2d31;
            --background-secondary-alt: #1e1f22;
            --background-floating: #111214;
            --channeltextarea-background: #383a40;
            --text-normal: #dbdee1;
            --text-muted: #949ba4;
            --header-primary: #f2f3f5;
            --header-secondary: #b5bac1;
            --brand-experiment: #5865f2;
            --brand-560: #4752c4;
            --status-positive: #23a55a;
            --status-danger: #f23f43;
            --text-link: #00a8fc;
            --border-subtle: #3f4147;
            --font-primary: "gg sans", "Noto Sans", sans-serif;
        }
        .theme-light {
            --background-primary: #ffffff;
            --background-secondary: #f2f3f5;
            --text-normal: #313338;
        }
        "#;
        let theme = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0x31, 0x33, 0x38));
        assert_eq!(theme.palette.sidebar, Rgba::rgb(0x2b, 0x2d, 0x31));
        assert_eq!(theme.palette.accent, Rgba::rgb(0x58, 0x65, 0xf2));
        assert_eq!(theme.palette.accent_hover, Rgba::rgb(0x47, 0x52, 0xc4));
        assert_eq!(theme.palette.text, Rgba::rgb(0xdb, 0xde, 0xe1));
        assert_eq!(theme.look.font_family.as_deref(), Some("gg sans"));
        let light = load_str(css, None, &Options::with_mode(Mode::Light)).unwrap();
        assert_eq!(light.palette.chat, Rgba::rgb(0xff, 0xff, 0xff));
        assert_eq!(light.palette.text, Rgba::rgb(0x31, 0x33, 0x38));
    }

    #[test]
    fn cascade_layers_beat_specificity_and_unlayered_wins_over_all() {
        let css = r#"
        @layer base {
            .theme-dark:root { --background-primary: #111111; --background-secondary: #444444; }
        }
        @layer theme {
            :root { --background-primary: #222222; }
        }
        :root { --background-secondary: #333333; }
        "#;
        let theme = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0x22, 0x22, 0x22));
        assert_eq!(theme.palette.sidebar, Rgba::rgb(0x33, 0x33, 0x33));
    }

    #[test]
    fn auto_picks_dark_for_a_dark_ground_and_light_for_a_light_one() {
        let dark = load_str(
            ":root{--background-primary:#111;--text-normal:#eee}",
            None,
            &Options::default(),
        )
        .unwrap();
        assert!(dark.palette.dark);
        let light = load_str(
            ":root{--background-primary:#fff;--text-normal:#111}",
            None,
            &Options::default(),
        )
        .unwrap();
        assert!(!light.palette.dark);
    }

    #[test]
    fn option_overrides_reach_the_palette_and_the_controls() {
        let mut options = Options::with_mode(Mode::Dark);
        options.overrides = vec![
            ("--rl-accent".into(), "#3ddc97".into()),
            ("--rl-scanline-opacity".into(), "0.5".into()),
        ];
        let theme = load_str(RECORD, None, &options).unwrap();
        assert_eq!(theme.palette.accent, Rgba::rgb(0x3d, 0xdc, 0x97));
        assert_eq!(theme.look.effects.scanlines, 0.5);
        let accent = theme
            .options
            .iter()
            .find(|option| option.name == "--rl-accent")
            .unwrap();
        assert_eq!(accent.value, "#3ddc97");
        let scanlines = theme
            .options
            .iter()
            .find(|option| option.name == "--rl-scanline-opacity")
            .unwrap();
        assert_eq!(scanlines.value, "0.5");
        assert!(matches!(
            scanlines.kind,
            crate::OptionKind::Percent {
                value: 50.0,
                percent: false
            }
        ));
    }

    #[test]
    fn an_accent_override_recolors_the_palette() {
        let mut options = Options::with_mode(Mode::Dark);
        options.accent = Some(Rgba::rgb(0x3d, 0xdc, 0x97));
        let theme = load_str(RECORD, None, &options).unwrap();
        assert_eq!(theme.palette.accent, Rgba::rgb(0x3d, 0xdc, 0x97));
    }

    #[test]
    fn light_dark_follows_the_chosen_appearance() {
        let css = r#"
        :root { color-scheme: light dark;
            --background-primary: light-dark(#fff8f7, #1a1111);
            --text-normal: light-dark(#221a19, #f1dedd);
            --brand-experiment: #904a46; }
        .theme-light { color-scheme: light; }
        "#;
        let dark = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(dark.palette.chat, Rgba::rgb(0x1a, 0x11, 0x11));
        assert_eq!(dark.palette.text, Rgba::rgb(0xf1, 0xde, 0xdd));
        let light = load_str(css, None, &Options::with_mode(Mode::Light)).unwrap();
        assert_eq!(light.palette.chat, Rgba::rgb(0xff, 0xf8, 0xf7));
        assert_eq!(light.palette.text, Rgba::rgb(0x22, 0x1a, 0x19));
    }

    #[test]
    fn discord_recolors_triplet_tokens_become_colours() {
        let css = r#"
        :root {
            --backgroundprimary: 30, 30, 30;
            --backgroundsecondary: 20, 20, 20;
            --backgroundsecondaryalt: 15, 15, 15;
            --backgroundfloating: 0, 0, 0;
            --textbrightest: 255, 255, 255;
            --accentcolor: 255, 179, 174;
            --font: "Roboto Condensed", sans-serif;
        }
        "#;
        let theme = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0x1e, 0x1e, 0x1e));
        assert_eq!(theme.palette.sidebar, Rgba::rgb(0x14, 0x14, 0x14));
        assert_eq!(theme.palette.rail, Rgba::rgb(0x0f, 0x0f, 0x0f));
        assert_eq!(theme.palette.raised, Rgba::rgb(0, 0, 0));
        assert_eq!(theme.palette.text, Rgba::rgb(255, 255, 255));
        assert_eq!(theme.palette.accent, Rgba::rgb(255, 179, 174));
        assert_eq!(theme.look.font_family.as_deref(), Some("Roboto Condensed"));
    }

    #[test]
    fn the_dark_rule_after_root_wins_and_root_after_dark_wins() {
        let after_root = load_str(
            ":root { --background-primary: #111111; } .theme-dark { --background-primary: #222222; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert_eq!(after_root.palette.chat, Rgba::rgb(0x22, 0x22, 0x22));
        let after_dark = load_str(
            ".theme-dark { --background-primary: #222222; } :root { --background-primary: #111111; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert_eq!(after_dark.palette.chat, Rgba::rgb(0x11, 0x11, 0x11));
    }

    #[test]
    fn an_important_rule_beats_higher_specificity() {
        let theme = load_str(
            ":root { --background-primary: #111111 !important; } \
             #app-mount { --background-primary: #222222; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0x11, 0x11, 0x11));
    }

    #[test]
    fn the_bundled_ambxst_theme_reads_as_its_palette() {
        let css = include_str!("../../../themes/Ambxst.theme.css");
        let dark = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(dark.name, "Ambxst");
        assert_eq!(dark.palette.chat, Rgba::rgb(0x1a, 0x11, 0x11));
        assert_eq!(dark.palette.sidebar, Rgba::rgb(0x23, 0x19, 0x19));
        assert_eq!(dark.palette.accent, Rgba::rgb(0xff, 0xb3, 0xae));
        assert_eq!(dark.palette.text, Rgba::rgb(0xf1, 0xde, 0xdd));
        assert_eq!(dark.look.radius, Some(16.0));
        assert_eq!(dark.look.font_family.as_deref(), Some("Roboto Condensed"));
        assert_eq!(dark.native.rail, Some(Rgba::rgb(0x14, 0x0c, 0x0c)));
        let light = load_str(css, None, &Options::with_mode(Mode::Light)).unwrap();
        assert_eq!(light.palette.chat, Rgba::rgb(0xff, 0xf8, 0xf7));
        assert_eq!(light.palette.accent, Rgba::rgb(0x90, 0x4a, 0x46));
    }

    #[test]
    fn the_bundled_record_theme_keeps_its_sidebar_background() {
        let css = include_str!("../../../themes/ReCord.theme.css");
        let theme = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
        assert_eq!(theme.native.sidebar, Some(Rgba::rgb(0x0e, 0x10, 0x0c)));
        assert_ne!(theme.native.sidebar, Some(Rgba::TRANSPARENT));
    }

    #[test]
    fn remote_imports_and_their_nested_imports_are_downloaded() {
        let mut calls = Vec::new();
        let css =
            "@import \"https://example.test/base/theme.css\";\n:root { --text-normal: #ffffff; }";
        let theme = load_str_with_imports(css, None, &Options::with_mode(Mode::Dark), &mut |url| {
            calls.push(url.to_owned());
            match url {
                "https://example.test/base/theme.css" => Some(
                    b"@import \"../shared/colors.css\"; :root { --text-normal: #ffffff; }".to_vec(),
                ),
                "https://example.test/shared/colors.css" => Some(
                    b":root { --background-primary: #123456; --brand-experiment: #abcdef; }"
                        .to_vec(),
                ),
                _ => None,
            }
        })
        .unwrap();
        assert_eq!(
            calls,
            [
                "https://example.test/base/theme.css",
                "https://example.test/shared/colors.css"
            ]
        );
        assert_eq!(theme.palette.chat, Rgba::rgb(0x12, 0x34, 0x56));
        assert_eq!(theme.palette.accent, Rgba::rgb(0xab, 0xcd, 0xef));
        assert_eq!(theme.palette.text, Rgba::rgb(255, 255, 255));
        assert!(theme.warnings.is_empty(), "{:?}", theme.warnings);
    }

    #[test]
    fn a_remote_font_is_downloaded_with_its_css() {
        let css = r#"
        @font-face {
            font-family: "Remote";
            font-weight: 700;
            src: url("https://example.test/remote.ttf") format("truetype");
        }
        :root { --background-primary: #111111; --text-normal: #eeeeee; }
        "#;
        let theme = load_str_with_imports(css, None, &Options::with_mode(Mode::Dark), &mut |url| {
            (url == "https://example.test/remote.ttf").then(|| vec![1, 2, 3, 4])
        })
        .unwrap();
        assert_eq!(theme.fonts.len(), 1);
        assert_eq!(theme.fonts[0].family, "Remote");
        assert_eq!(theme.fonts[0].weight, 700);
        assert_eq!(theme.fonts[0].bytes, [1, 2, 3, 4]);
        assert!(theme.warnings.is_empty(), "{:?}", theme.warnings);
    }

    #[test]
    fn a_remote_import_without_a_fetcher_warns_and_keeps_the_local_theme() {
        let theme = load_str(
            "@import \"https://example.test/theme.css\"; :root { --background-primary: #101010; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert_eq!(theme.palette.chat, Rgba::rgb(0x10, 0x10, 0x10));
        assert!(
            theme
                .warnings
                .iter()
                .any(|warning| warning.contains("could not read the @import")),
            "{:?}",
            theme.warnings
        );
    }

    #[test]
    fn options_unique_to_the_other_appearance_are_still_offered() {
        let theme = load_str(
            ":root { --background-primary: #111111; } \
             .theme-light { --paper-only: #123456; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert!(
            theme
                .options
                .iter()
                .any(|option| option.name == "--paper-only"
                    && matches!(option.kind, crate::OptionKind::Color { .. })),
            "{:?}",
            theme.options
        );
    }

    #[test]
    fn fonts_and_pictures_beside_an_imported_file_use_its_folder() {
        let folder = std::env::temp_dir().join(format!(
            "recore-css-test-{}-{}",
            std::process::id(),
            "relative-assets"
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("sub")).unwrap();
        fs::write(
            folder.join("main.theme.css"),
            "@import \"sub/one.css\"; :root { --background-primary: #111111; }",
        )
        .unwrap();
        fs::write(
            folder.join("sub/one.css"),
            r#"
            @font-face {
                font-family: "Beside";
                src: local("Missing Local"), url("font.ttf") format("truetype");
            }
            :root { --background-image: url("picture.png"); }
            "#,
        )
        .unwrap();
        fs::write(folder.join("sub/font.ttf"), [9, 8, 7]).unwrap();
        fs::write(folder.join("sub/picture.png"), b"not decoded here").unwrap();
        let theme = load(folder.join("main.theme.css")).unwrap();
        assert_eq!(theme.fonts.len(), 1);
        assert_eq!(theme.fonts[0].family, "Beside");
        assert_eq!(theme.fonts[0].bytes, [9, 8, 7]);
        assert_eq!(
            theme.look.background,
            Some(Image::File(folder.join("sub/picture.png")))
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_remote_import_resolves_its_relative_fonts_and_pictures() {
        let css = "@import \"https://example.test/base/theme.css\";";
        let theme =
            load_str_with_imports(
                css,
                None,
                &Options::with_mode(Mode::Dark),
                &mut |url| match url {
                    "https://example.test/base/theme.css" => Some(
                        br#"
                    @font-face {
                        font-family: "Remote beside";
                        src: url("fonts/remote.ttf") format("truetype");
                    }
                    :root {
                        --background-primary: #101010;
                        --background-image: url("images/wp.png");
                    }
                    "#
                        .to_vec(),
                    ),
                    "https://example.test/base/fonts/remote.ttf" => Some(vec![4, 5, 6]),
                    _ => None,
                },
            )
            .unwrap();
        assert_eq!(theme.fonts.len(), 1);
        assert_eq!(theme.fonts[0].bytes, [4, 5, 6]);
        assert_eq!(
            theme.look.background,
            Some(Image::Url("https://example.test/base/images/wp.png".into()))
        );
        assert!(theme.warnings.is_empty(), "{:?}", theme.warnings);
    }

    #[test]
    fn a_bare_tag_does_not_paint_a_region() {
        let theme = load_str(
            "div { background: #123456; }",
            None,
            &Options::with_mode(Mode::Dark),
        )
        .unwrap();
        assert_eq!(theme.native.rail, None);
        assert_eq!(theme.native.chat, None);
    }

    #[test]
    fn the_bundled_record_theme_reads_as_recores_palette() {
        let css = include_str!("../../../themes/ReCord.theme.css");
        let theme = load_str(css, None, &Options::default()).unwrap();
        assert_eq!(theme.name, "ReCord");
        assert!(theme.palette.dark);
        assert_eq!(theme.palette.chat, Rgba::rgb(0x08, 0x09, 0x07));
        assert_eq!(theme.palette.text, Rgba::rgb(0xe4, 0xcd, 0xb1));
        assert_eq!(theme.palette.accent, Rgba::rgb(0xe9, 0x42, 0x21));
        assert_eq!(theme.palette.accent_hover, Rgba::rgb(0xed, 0x65, 0x45));
        assert_eq!(theme.look.font_family.as_deref(), Some("Pixel Operator"));
        assert!(
            theme
                .fonts
                .iter()
                .any(|font| font.family == "Pixel Operator" && font.weight == 700)
        );
    }

    #[test]
    fn record_css_exposes_its_options() {
        let css = include_str!("../../../themes/ReCord.theme.css");
        let theme = load_str(css, None, &Options::default()).unwrap();
        let find = |name: &str| {
            theme
                .options
                .iter()
                .find(|option| option.name == name)
                .unwrap_or_else(|| panic!("{name} is offered; got {:?}", theme.options.len()))
        };
        assert!(matches!(
            find("--rl-accent").kind,
            crate::OptionKind::Color { .. }
        ));
        assert!(matches!(
            find("--rl-motion").kind,
            crate::OptionKind::Choice { .. }
        ));
        assert!(matches!(
            find("--rl-scanline-opacity").kind,
            crate::OptionKind::Percent { .. }
        ));
        assert!(matches!(
            find("--rl-background-image").kind,
            crate::OptionKind::Image { .. }
        ));
        assert!(
            theme.options.len() > 40,
            "ReCord has many options: {}",
            theme.options.len()
        );
    }

    #[test]
    fn imports_are_read_beside_the_file_that_names_them() {
        let folder = std::env::temp_dir().join(format!(
            "recore-css-test-{}-{}",
            std::process::id(),
            "imports"
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("sub")).unwrap();
        fs::write(
            folder.join("main.theme.css"),
            "@import \"sub/one.css\"; :root { --background-primary: #111; }",
        )
        .unwrap();
        fs::write(
            folder.join("sub/one.css"),
            "@import \"two.css\"; :root { --text-normal: #eee; }",
        )
        .unwrap();
        fs::write(
            folder.join("sub/two.css"),
            ":root { --brand-experiment: #123456; }",
        )
        .unwrap();
        let theme = load(folder.join("main.theme.css")).unwrap();
        assert_eq!(theme.palette.accent, Rgba::rgb(0x12, 0x34, 0x56));
        assert_eq!(theme.palette.chat, Rgba::rgb(0x11, 0x11, 0x11));
        assert!(theme.warnings.is_empty(), "{:?}", theme.warnings);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn bundled_css_themes_leave_nothing_in_other() {
        for name in ["ReCord.theme.css", "Ambxst.theme.css"] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../themes")
                .join(name);
            let theme = load(path).unwrap();
            let other: Vec<_> = theme
                .options
                .iter()
                .filter(|option| option.group == "Other")
                .collect();
            assert!(other.is_empty(), "{}: {other:?}", theme.name);
        }
    }

    #[test]
    fn a_theme_in_a_subfolder_is_read_and_a_duplicate_is_not() {
        let folder = std::env::temp_dir().join(format!(
            "recore-css-test-{}-{}",
            std::process::id(),
            "nested"
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("pack/themes")).unwrap();
        let css = ":root { --background-primary: #123456; }";
        fs::write(folder.join("pack/themes/nested.theme.css"), css).unwrap();
        fs::write(folder.join("same.theme.css"), css).unwrap();
        let scan = scan(std::slice::from_ref(&folder));
        assert_eq!(scan.themes.len(), 1, "{:?}", scan.problems);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_missing_theme_folder_is_not_a_problem() {
        let folder = std::env::temp_dir().join(format!(
            "recore-css-test-{}-{}",
            std::process::id(),
            "missing"
        ));
        let scan = scan(&[folder]);
        assert!(scan.themes.is_empty());
        assert!(scan.problems.is_empty(), "{:?}", scan.problems);
    }

    #[test]
    fn themes_in_a_folder_are_read_and_named() {
        let folder =
            std::env::temp_dir().join(format!("recore-css-test-{}-{}", std::process::id(), "scan"));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("one.theme.css"), RECORD).unwrap();
        fs::write(
            folder.join("two.theme.css"),
            ":root{--background-primary:#123456}",
        )
        .unwrap();
        let scan = scan(std::slice::from_ref(&folder));
        assert_eq!(scan.themes.len(), 2);
        assert_eq!(scan.themes[0].name, "ReCord");
        fs::remove_dir_all(&folder).unwrap();
    }
}
