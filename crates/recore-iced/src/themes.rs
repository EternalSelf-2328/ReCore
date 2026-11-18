//! The themes the window shows: ReCore's own (ReCord's palette, dark or
//! paper), the person's custom ones in `layout.json`, and the Discord CSS
//! themes Vencord, Vesktop and their kin keep. The choice is `layout.json`'s
//! `theme`: a built-in id, a `custom:` one or a `css:` one. See
//! docs/themes.md.
//!
//! The chosen theme is applied before the window's first frame, and again to
//! a window made anew. With none kept, the window shows ReCore. An id ReCore
//! doesn't know, or a deleted theme's, shows ReCore too. The window-control
//! command `theme ID` chooses and keeps one as Appearance does; Appearance
//! itself is Settings' (`settings/appearance.rs`).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use iced::Color;
use recore_core::theme::{self as palette, Colour, Custom};
use recore_css::{NativeStyle, ThemeFile};

use crate::{
    fonts,
    layout::{self, Layout},
    theme::Theme,
};

/// The theme shown when `layout.json` names none: ReCord's terminal palette.
const DEFAULT: &str = "recore";

/// The themes this data folder offers and the one it chose.
/// ReCore's own look, the knobs ReCord's plugin exposes: its CRT levels,
/// its window picture, how opaque the panels over it are, and its type.
/// A CSS theme keeps its own values instead.
#[derive(Clone, Debug)]
pub struct RecoreOptions {
    pub scanlines: f32,
    pub grain: f32,
    pub vignette: f32,
    pub frame: f32,
    pub beam: bool,
    pub flicker: bool,
    pub avatar_filter: bool,
    pub background: Option<PathBuf>,
    pub background_dim: f32,
    pub panel_opacity: f32,
    pub font: String,
    pub motion: Motion,
}

/// ReCord's three motion levels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Full,
    Subtle,
    Off,
}

impl Default for RecoreOptions {
    fn default() -> Self {
        Self {
            scanlines: 0.25,
            grain: 0.18,
            vignette: 0.55,
            frame: 0.6,
            beam: true,
            flicker: false,
            avatar_filter: true,
            background: None,
            background_dim: 0.5,
            panel_opacity: 0.75,
            font: "pixel".into(),
            motion: Motion::Full,
        }
    }
}

impl RecoreOptions {
    fn read(layout: &Layout) -> Self {
        let percent = |value: Option<u8>, fallback: f32| {
            value
                .map(|value| f32::from(value.min(100)) / 100.0)
                .unwrap_or(fallback)
        };
        Self {
            scanlines: percent(layout.recore_scanlines, 0.25),
            grain: percent(layout.recore_grain, 0.18),
            vignette: percent(layout.recore_vignette, 0.55),
            frame: if layout.recore_frame.unwrap_or(true) {
                0.6
            } else {
                0.0
            },
            beam: layout.recore_beam.unwrap_or(true),
            flicker: layout.recore_flicker.unwrap_or(false),
            avatar_filter: layout.recore_avatar_filter.unwrap_or(true),
            background: layout.recore_background.as_deref().map(PathBuf::from),
            background_dim: percent(layout.recore_background_dim, 0.5),
            panel_opacity: percent(layout.recore_panel_opacity, 0.75),
            font: layout.recore_font.clone().unwrap_or_else(|| "pixel".into()),
            motion: match layout.recore_motion.as_deref() {
                Some("subtle") => Motion::Subtle,
                Some("off") => Motion::Off,
                _ => Motion::Full,
            },
        }
    }
}

pub struct Themes {
    pub custom: Vec<Custom>,
    /// Discord CSS themes read from the folders below.
    pub css: Vec<ThemeFile>,
    /// The theme files that could not be read, for Appearance to list.
    pub problems: Vec<recore_css::Problem>,
    /// Where ReCore looked for CSS themes.
    pub folders: Vec<PathBuf>,
    pub chosen: Option<String>,
    /// ReCore's accent, over its theme's own signal orange.
    pub accent: Option<Colour>,
    /// Whether the CRT layer is drawn, and whether low power takes its grain.
    pub effects: bool,
    pub low_power: bool,
    /// ReCore's own options, when ReCore or ReCore Paper is chosen.
    pub recore: RecoreOptions,
    /// The values the person changed in CSS themes, by theme id and custom
    /// property, kept in `layout.json`.
    pub overrides: BTreeMap<String, BTreeMap<String, String>>,
    /// The chosen background picture's first frame, and how far it is
    /// dimmed. A GIF or video plays from `background_picture` through the
    /// window's animation players.
    pub background: Option<iced::widget::image::Handle>,
    pub background_picture: Option<std::sync::Arc<recore_core::chat::Picture>>,
    pub background_dim: f32,
}

impl Themes {
    /// The themes `layout` keeps, and the CSS themes in `root`'s themes
    /// folder and the clients' own.
    pub fn read(root: &Path, layout: &Layout) -> Self {
        let mut custom = layout.custom_themes.clone();
        palette::tidy(&mut custom);
        let folders = theme_folders(root);
        let accent = layout.recore_accent.as_deref().and_then(palette::parse);
        let effects = layout.recore_effects.unwrap_or(true);
        let low_power = layout.recore_low_power.unwrap_or(false);
        let recore = RecoreOptions::read(layout);
        let overrides = layout.css_options.clone();
        let mut themes = Self {
            custom,
            css: Vec::new(),
            problems: Vec::new(),
            folders,
            chosen: layout.theme.clone(),
            accent,
            effects,
            low_power,
            recore,
            overrides,
            background: None,
            background_picture: None,
            background_dim: 0.0,
        };
        themes.reload();
        themes
    }

    /// Reads the CSS themes again, keeping the chosen one if it's still there.
    pub fn reload(&mut self) {
        let scan = recore_css::scan(&self.folders);
        self.css = scan.themes;
        self.problems = scan.problems;
        self.apply_overrides();
        let chosen = self.chosen_id().to_owned();
        self.apply_font(&chosen);
        self.load_background();
    }

    /// Re-reads each CSS theme that has options the person changed, so its
    /// palette and effects show them.
    fn apply_overrides(&mut self) {
        if self.overrides.is_empty() {
            return;
        }
        for theme in &mut self.css {
            let Some(changed) = self.overrides.get(&theme.id) else {
                continue;
            };
            if changed.is_empty() {
                continue;
            }
            let Some(path) = theme.path.clone() else {
                continue;
            };
            let options = recore_css::Options {
                overrides: changed
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect(),
                ..recore_css::Options::default()
            };
            if let Ok(reloaded) = recore_css::load_with(path, &options) {
                *theme = reloaded;
            }
        }
    }

    /// Whether avatars are square and framed, as ReCord draws them: ReCore's
    /// own themes, and a CSS theme whose radius is square.
    pub fn square_avatars(&self) -> bool {
        if matches!(self.chosen_id(), "recore" | "recore-light") {
            return self.recore.avatar_filter;
        }
        self.chosen_css()
            .is_some_and(|css| css.look.radius.unwrap_or(8.0) < 2.0)
    }

    /// Whether avatars get ReCord's warm grade and scanlines: ReCore's own
    /// themes, and a CSS theme that names an avatar filter or scanline.
    pub fn avatar_filter(&self) -> bool {
        if self.low_power {
            return false;
        }
        if matches!(self.chosen_id(), "recore" | "recore-light") {
            return true;
        }
        self.chosen_css().is_some_and(|css| {
            css.look.effects.avatar_scanline > 0.0
                || css
                    .look
                    .effects
                    .avatar_filter
                    .as_deref()
                    .is_some_and(|filter| {
                        let filter = filter.trim();
                        !filter.is_empty() && !filter.eq_ignore_ascii_case("none")
                    })
        })
    }

    /// The chosen theme if it is a CSS one, for Appearance's options.
    pub fn chosen_css(&self) -> Option<&ThemeFile> {
        self.css.iter().find(|css| css.id == self.chosen_id())
    }

    /// Keeps `value` for `name` in the CSS theme `id`, and shows it.
    pub fn set_option(&mut self, root: &Path, id: &str, name: &str, value: String) {
        let changed = self.overrides.entry(id.to_owned()).or_default();
        changed.insert(name.to_owned(), value);
        layout::store(root, "css_options", &self.overrides);
        self.reload_one(id);
    }

    /// Forgets every changed option of the CSS theme `id`.
    pub fn clear_options(&mut self, root: &Path, id: &str) {
        self.overrides.remove(id);
        layout::store(root, "css_options", &self.overrides);
        self.reload_one(id);
        let chosen = self.chosen_id().to_owned();
        self.apply_font(&chosen);
    }

    /// Re-reads one CSS theme with its changed options.
    fn reload_one(&mut self, id: &str) {
        let Some(index) = self.css.iter().position(|css| css.id == id) else {
            return;
        };
        let Some(path) = self.css[index].path.clone() else {
            return;
        };
        let options = recore_css::Options {
            overrides: self
                .overrides
                .get(id)
                .map(|changed| {
                    changed
                        .iter()
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            ..recore_css::Options::default()
        };
        if let Ok(reloaded) = recore_css::load_with(path, &options) {
            self.css[index] = reloaded;
        }
        let chosen = self.chosen_id().to_owned();
        self.apply_font(&chosen);
        self.load_background();
    }

    /// The theme to show.
    pub fn shown(&self) -> Theme {
        self.find(self.chosen_id()).unwrap_or_else(|| {
            let built_in = palette::find(DEFAULT).expect("ReCore is a built-in theme");
            theme(built_in.id, &built_in.theme)
        })
    }

    /// The chosen theme's id, as Appearance marks it.
    pub fn chosen_id(&self) -> &str {
        if let Some(id) = self.chosen.as_deref()
            && self.find(id).is_some()
        {
            return id;
        }
        DEFAULT
    }

    /// Lowers the panels over ReCore's own background picture, as ReCord's
    /// surface opacity does. Text and lines keep their full strength.
    fn paint_recore(&self, shown: &mut Theme) {
        if !matches!(self.chosen_id(), "recore" | "recore-light")
            || self.background.is_none()
            || self.recore.panel_opacity >= 1.0
        {
            return;
        }
        let opacity = self.recore.panel_opacity.clamp(0.0, 1.0);
        for colour in [
            &mut shown.rail,
            &mut shown.sidebar,
            &mut shown.panel,
            &mut shown.chat,
            &mut shown.stage,
            &mut shown.raised,
            &mut shown.field,
            &mut shown.sunken,
            &mut shown.hover,
            &mut shown.active,
            &mut shown.row_hover,
            &mut shown.button_default,
            &mut shown.button_default_hover,
        ] {
            colour.a *= opacity;
        }
    }

    /// `id`'s theme, if it's one ReCore has, the person made, or a CSS theme
    /// in one of the folders.
    pub fn find(&self, id: &str) -> Option<Theme> {
        if let Some(css) = self.css.iter().find(|css| css.id == id) {
            return Some(from_css(css));
        }
        palette::lookup(id, &self.custom).map(|(id, found)| {
            let found = match (id, self.accent) {
                ("recore" | "recore-light", Some(accent)) => found.accented(accent),
                _ => found,
            };
            let mut shown = theme(id, &found);
            shown.square = matches!(id, "recore" | "recore-light");
            self.paint_recore(&mut shown);
            shown.worked_out()
        })
    }

    /// Keeps `accent` as ReCore's own accent, or clears it with `None`.
    pub fn set_accent(&mut self, root: &Path, accent: Option<Colour>) {
        self.accent = accent;
        layout::store(root, "recore_accent", accent.map(palette::hex));
    }

    /// Turns the CRT layer on or off, and keeps it.
    pub fn set_effects(&mut self, root: &Path, on: bool) {
        self.effects = on;
        layout::store(root, "recore_effects", on);
    }

    /// Turns low power on or off, and keeps it.
    pub fn set_low_power(&mut self, root: &Path, on: bool) {
        self.low_power = on;
        layout::store(root, "recore_low_power", on);
    }

    /// ReCore's CRT knobs, kept in `layout.json`.
    pub fn set_scanlines(&mut self, root: &Path, percent: u8) {
        self.recore.scanlines = f32::from(percent.min(100)) / 100.0;
        layout::store(root, "recore_scanlines", percent.min(100));
    }

    pub fn set_grain(&mut self, root: &Path, percent: u8) {
        self.recore.grain = f32::from(percent.min(100)) / 100.0;
        layout::store(root, "recore_grain", percent.min(100));
    }

    pub fn set_vignette(&mut self, root: &Path, percent: u8) {
        self.recore.vignette = f32::from(percent.min(100)) / 100.0;
        layout::store(root, "recore_vignette", percent.min(100));
    }

    pub fn set_beam(&mut self, root: &Path, on: bool) {
        self.recore.beam = on;
        layout::store(root, "recore_beam", on);
    }

    pub fn set_flicker(&mut self, root: &Path, on: bool) {
        self.recore.flicker = on;
        layout::store(root, "recore_flicker", on);
    }

    pub fn set_frame(&mut self, root: &Path, on: bool) {
        self.recore.frame = if on { 0.6 } else { 0.0 };
        layout::store(root, "recore_frame", on);
    }

    pub fn set_avatar_filter(&mut self, root: &Path, on: bool) {
        self.recore.avatar_filter = on;
        layout::store(root, "recore_avatar_filter", on);
    }

    pub fn set_background(&mut self, root: &Path, path: Option<PathBuf>) {
        let kept = path.map(|path| path.display().to_string());
        self.recore.background = kept.as_deref().map(PathBuf::from);
        layout::store(root, "recore_background", kept);
        self.load_background();
    }

    pub fn set_background_dim(&mut self, root: &Path, percent: u8) {
        self.recore.background_dim = f32::from(percent.min(100)) / 100.0;
        layout::store(root, "recore_background_dim", percent.min(100));
    }

    pub fn set_panel_opacity(&mut self, root: &Path, percent: u8) {
        self.recore.panel_opacity = f32::from(percent.min(100)) / 100.0;
        layout::store(root, "recore_panel_opacity", percent.min(100));
    }

    pub fn set_font(&mut self, root: &Path, name: &str) {
        self.recore.font = name.to_owned();
        layout::store(root, "recore_font", name.to_owned());
        let chosen = self.chosen_id().to_owned();
        self.apply_font(&chosen);
    }

    pub fn set_motion(&mut self, root: &Path, motion: Motion) {
        self.recore.motion = motion;
        let name = match motion {
            Motion::Full => "full",
            Motion::Subtle => "subtle",
            Motion::Off => "off",
        };
        layout::store(root, "recore_motion", name);
    }

    /// The chosen theme's motion level.
    pub fn motion(&self) -> Motion {
        if matches!(self.chosen_id(), "recore" | "recore-light") {
            return self.recore.motion;
        }
        match self.chosen_css().map(|css| css.look.motion) {
            Some(recore_css::Motion::Off) => Motion::Off,
            Some(recore_css::Motion::Subtle) => Motion::Subtle,
            _ => Motion::Full,
        }
    }

    /// Whether the CRT layer has a beam or flicker that needs the clock.
    pub fn animates_effects(&self) -> bool {
        self.shade(&Theme::new("probe", true))
            .is_some_and(|shade| shade.beam > 0.0 || shade.flicker > 0.0)
    }

    /// The CRT layer the window draws over itself: ReCord's effect levels for
    /// a ReCore theme, the CSS theme's own `--rl-*` levels otherwise. `None`
    /// when the layer is off or the theme asks for no effects.
    pub fn shade(&self, theme: &Theme) -> Option<recore_iced_kit::effects::Shade> {
        if !self.effects {
            return None;
        }
        let effects = if let Some(css) = self.css.iter().find(|css| css.id == self.chosen_id()) {
            css.look.effects.clone()
        } else if matches!(self.chosen_id(), "recore" | "recore-light") {
            recore_css::Effects {
                scanlines: self.recore.scanlines,
                grain: self.recore.grain,
                vignette: self.recore.vignette,
                flicker: self.recore.flicker,
                beam: self.recore.beam,
                frame: self.recore.frame,
                avatar_filter: None,
                avatar_scanline: if self.recore.avatar_filter { 0.14 } else { 0.0 },
            }
        } else {
            return None;
        };
        let grain = if self.low_power { 0.0 } else { effects.grain };
        let moving = self.motion() != Motion::Off;
        let shade = recore_iced_kit::effects::Shade {
            scanlines: effects.scanlines,
            grain,
            vignette: effects.vignette,
            frame: effects.frame,
            beam: if self.low_power || !effects.beam || !moving {
                0.0
            } else {
                0.5
            },
            flicker: if self.low_power || !effects.flicker || !moving {
                0.0
            } else {
                1.0
            },
            line: theme.line,
            accent: theme.accent,
            phase: 0.0,
        };
        shade.visible().then_some(shade)
    }

    /// Keeps `id` as the chosen theme in `layout.json`, leaving the rest of
    /// what's kept there as it is, and gives the window its font.
    pub fn choose(&mut self, root: &Path, id: &str) {
        self.chosen = Some(id.to_owned());
        layout::store(root, "theme", id);
        self.apply_font(id);
        self.load_background();
    }

    /// Decodes the chosen CSS theme's background picture, if it names one
    /// ReCore can read, and keeps how far it is dimmed.
    fn load_background(&mut self) {
        if matches!(self.chosen_id(), "recore" | "recore-light") {
            self.set_background_picture(
                self.recore
                    .background
                    .as_deref()
                    .and_then(|path| load_picture_file(path)),
            );
            self.background_dim = self.recore.background_dim;
            return;
        }
        let (dim, background) = {
            let Some(css) = self.chosen_css() else {
                self.background = None;
                self.background_dim = 0.0;
                return;
            };
            let dim = css.look.background_dim.unwrap_or(0.0);
            let background = match &css.look.background {
                Some(recore_css::Image::Data(bytes)) => {
                    recore_core::chat::decode_picture(bytes, BACKGROUND_WIDTH, BACKGROUND_HEIGHT)
                }
                Some(recore_css::Image::File(path)) => load_picture_file(path),
                _ => None,
            };
            (dim, background)
        };
        self.background_dim = dim;
        self.set_background_picture(background);
    }

    /// Keeps a decoded background and the handle that shows its first frame
    /// until an animation's player begins.
    fn set_background_picture(&mut self, picture: Option<recore_core::chat::Picture>) {
        self.background = picture.as_ref().map(|picture| {
            iced::widget::image::Handle::from_rgba(
                picture.width,
                picture.height,
                picture.rgba.to_vec(),
            )
        });
        self.background_picture = picture.map(std::sync::Arc::new);
    }

    /// Loads the fonts a theme carries and draws with its family.
    fn apply_font(&self, id: &str) {
        let based_on_recore = self
            .custom
            .iter()
            .find(|custom| custom.id == id)
            .is_some_and(|custom| matches!(custom.base, "recore" | "recore-light"));
        if matches!(id, "recore" | "recore-light") || based_on_recore {
            fonts::set_family(recore_family(&self.recore.font));
            return;
        }
        if let Some(css) = self.css.iter().find(|css| css.id == id) {
            for font in &css.fonts {
                fonts::load(&font.family, font.bytes.clone());
            }
            if let Some(family) = &css.look.font_family {
                fonts::set_family(Some(family));
                return;
            }
        }
        fonts::set_family(None);
    }

    /// Where the themes folder is, making it if it isn't there.
    pub fn themes_folder(&self) -> &Path {
        &self.folders[0]
    }
}

/// Gives the window the chosen theme's font before it draws its first frame
/// (the `default_font` iced lays un-fonted text with is read then), so a CSS
/// theme's type or ReCord's pixel type covers even text that names no font.
pub fn prepare_fonts(root: &Path) {
    let layout = Layout::read(root);
    if matches!(layout.theme.as_deref(), Some("recore" | "recore-light")) {
        fonts::set_family(recore_family(
            layout.recore_font.as_deref().unwrap_or("pixel"),
        ));
        return;
    }
    let themes = Themes::read(root, &layout);
    themes.apply_font(themes.chosen_id());
}

/// How large a background is decoded: large enough for a desktop, small
/// enough that a GIF's frames and a video's RGBA buffer stay reasonable.
const BACKGROUND_WIDTH: u32 = 1920;
const BACKGROUND_HEIGHT: u32 = 1080;

/// Loads a background picture from a file. GIF, APNG, WebP and video files
/// keep their animation, which the window's picture players step.
fn load_picture_file(path: &Path) -> Option<recore_core::chat::Picture> {
    recore_core::chat::decode_picture_at(path, BACKGROUND_WIDTH, BACKGROUND_HEIGHT)
}

/// The family ReCore's own theme draws with.
fn recore_family(name: &str) -> Option<&'static str> {
    match name {
        "mono" => Some("Pixel Operator Mono"),
        "system" | "discord" => None,
        _ => Some("Pixel Operator"),
    }
}

/// The folders a theme may come from: this data folder's, then the desktop's
/// own client folders. A later file with the same id keeps its own id.
fn theme_folders(root: &Path) -> Vec<PathBuf> {
    let own = root.join("themes");
    let _ = std::fs::create_dir_all(&own);
    let mut folders = Vec::new();
    let mut seen = BTreeSet::new();
    let beside_the_binary = std::env::current_exe()
        .ok()
        .and_then(|binary| binary.parent().map(|folder| folder.join("themes")));
    // Running from the repository finds the bundled ReCord theme too; a
    // released binary doesn't take themes from whatever folder it was
    // started in.
    let here = PathBuf::from("themes");
    let here = cfg!(debug_assertions)
        .then_some(here)
        .filter(|folder| folder.is_dir());
    for folder in std::iter::once(own)
        .chain(beside_the_binary)
        .chain(here)
        .chain(recore_css::theme_folders())
    {
        let canonical = std::fs::canonicalize(&folder).unwrap_or_else(|_| folder.clone());
        if seen.insert(canonical) {
            folders.push(folder);
        }
    }
    folders
}

/// Fills the kit's theme from the core's.
macro_rules! from_palette {
    ($($name:ident,)*) => {
        /// The core's `theme`, named `id`, as the window paints with it.
        pub fn theme(id: &str, theme: &palette::Theme) -> Theme {
            let mut shown = Theme::new(id, theme.dark);
            $(shown.$name = color(theme.$name);)*
            shown.mention = color(theme.mention());
            shown.worked_out()
        }
    };
}

recore_iced_kit::for_theme_colours!(from_palette);

/// A CSS theme's colours as the window paints with them.
fn from_css(file: &ThemeFile) -> Theme {
    let theme = &file.palette;
    let mut shown = Theme::new(&file.id, theme.dark);
    shown.square = file.look.radius.unwrap_or(8.0) < 2.0;
    macro_rules! assign {
        ($($name:ident,)*) => {
            $(shown.$name = css_color(theme.$name);)*
        };
    }
    recore_iced_kit::for_theme_colours!(assign);
    shown.mention = css_color(theme.link);
    apply_kit_native(&mut shown, &file.native);
    shown.worked_out()
}

/// A CSS theme's colours as the core keeps them, for Appearance's tiles.
pub fn css_palette(file: &ThemeFile) -> palette::Theme {
    let theme = &file.palette;
    let mut found = palette::RECORE_DARK;
    found.dark = theme.dark;
    macro_rules! assign {
        ($($name:ident,)*) => {
            $(found.$name = core_colour(theme.$name);)*
        };
    }
    recore_iced_kit::for_theme_colours!(assign);
    apply_core_native(&mut found, &file.native);
    found
}

/// A selector-derived colour on the window's palette.
fn apply_kit_native(theme: &mut Theme, native: &NativeStyle) {
    macro_rules! set {
        ($($field:ident,)*) => {
            $(if let Some(colour) = native.$field {
                theme.$field = css_color(colour);
            })*
        };
    }
    set!(
        rail, sidebar, panel, chat, raised, field, text, secondary, muted, hover, row_hover,
    );
}

/// A selector-derived colour on the core's palette, for Appearance's tile.
fn apply_core_native(theme: &mut palette::Theme, native: &NativeStyle) {
    macro_rules! set {
        ($($field:ident,)*) => {
            $(if let Some(colour) = native.$field {
                theme.$field = core_colour(colour);
            })*
        };
    }
    set!(
        rail, sidebar, panel, chat, raised, field, text, secondary, muted, hover, row_hover,
    );
}

/// A core colour as iced's.
pub fn color(colour: Colour) -> Color {
    Color::from_rgba8(
        colour.red(),
        colour.green(),
        colour.blue(),
        f32::from(colour.alpha()) / 255.0,
    )
}

/// A CSS colour as iced's.
fn css_color(colour: recore_css::Rgba) -> Color {
    Color::from_rgba8(colour.r, colour.g, colour.b, f32::from(colour.a) / 255.0)
}

/// A CSS colour as the core's.
fn core_colour(colour: recore_css::Rgba) -> Colour {
    Colour::from_argb(
        (u32::from(colour.a) << 24)
            | (u32::from(colour.r) << 16)
            | (u32::from(colour.g) << 8)
            | u32::from(colour.b),
    )
}

/// Opens a folder in the desktop's file manager, for Appearance's button.
pub fn open_folder(path: &Path) {
    #[cfg(target_os = "linux")]
    let opener = "xdg-open";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(windows)]
    let opener = "explorer";
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LAYOUT;

    #[test]
    fn a_layout_without_a_theme_shows_recore_and_an_unknown_one_too() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(
            Themes::read(folder.path(), &Layout::read(folder.path()))
                .shown()
                .id,
            "recore"
        );
        std::fs::write(
            folder.path().join(LAYOUT),
            r#"{"theme": "gone", "zoom": 1.2}"#,
        )
        .unwrap();
        assert_eq!(
            Themes::read(folder.path(), &Layout::read(folder.path()))
                .shown()
                .id,
            "recore"
        );
    }

    #[test]
    fn choosing_a_theme_keeps_the_rest_of_the_layout() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(
            folder.path().join(LAYOUT),
            r##"{"zoom": 1.2, "custom_themes": [{"id": "custom:1", "name": "Plum",
                "base": "sakura", "colours": {"chat": "#2a1a3a"}}]}"##,
        )
        .unwrap();
        let mut themes = Themes::read(folder.path(), &Layout::read(folder.path()));
        let plum = themes.find("custom:1").unwrap();
        assert!(!plum.dark, "Sakura's");
        assert_eq!(plum.chat, crate::style::hex(0x2a1a3a));
        themes.choose(folder.path(), "custom:1");
        crate::layout::flush(folder.path());
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.path().join(LAYOUT)).unwrap()).unwrap();
        assert_eq!(written["theme"], "custom:1");
        assert_eq!(written["zoom"], 1.2);
        assert_eq!(
            Themes::read(folder.path(), &Layout::read(folder.path())).shown(),
            plum
        );
    }

    #[test]
    fn the_worked_out_colours_follow_the_theme() {
        let recore = Themes {
            custom: Vec::new(),
            css: Vec::new(),
            problems: Vec::new(),
            folders: Vec::new(),
            chosen: Some("recore".into()),
            accent: None,
            effects: true,
            low_power: false,
            recore: RecoreOptions::default(),
            overrides: BTreeMap::new(),
            background: None,
            background_picture: None,
            background_dim: 0.0,
        }
        .shown();
        assert_eq!(recore.accent, crate::style::hex(0xe94221));
        assert_eq!(recore.chat, crate::style::hex(0x080907));
        let light = Themes {
            chosen: Some("recore-light".into()),
            ..Themes {
                custom: Vec::new(),
                css: Vec::new(),
                problems: Vec::new(),
                folders: Vec::new(),
                chosen: None,
                accent: None,
                effects: true,
                low_power: false,
                recore: RecoreOptions::default(),
                overrides: BTreeMap::new(),
                background: None,
                background_picture: None,
                background_dim: 0.0,
            }
        }
        .shown();
        assert_eq!(light.chat, crate::style::hex(0xf3e9d7));
    }

    #[test]
    fn a_recore_accent_is_kept_and_recolors_the_theme() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(
            folder.path().join(LAYOUT),
            r##"{"theme": "recore", "recore_accent": "#3ddc97"}"##,
        )
        .unwrap();
        let mut themes = Themes::read(folder.path(), &Layout::read(folder.path()));
        assert_eq!(themes.shown().accent, crate::style::hex(0x3ddc97));
        themes.set_accent(
            folder.path(),
            Some(recore_core::theme::Colour::rgb(0x4fb8c9)),
        );
        crate::layout::flush(folder.path());
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.path().join(LAYOUT)).unwrap()).unwrap();
        assert_eq!(written["recore_accent"], "#4fb8c9");
        assert_eq!(
            Themes::read(folder.path(), &Layout::read(folder.path()))
                .shown()
                .accent,
            crate::style::hex(0x4fb8c9)
        );
    }

    #[test]
    fn a_css_theme_in_the_themes_folder_is_offered_and_paints() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(folder.path().join("themes")).unwrap();
        std::fs::write(
            folder.path().join("themes/plum.theme.css"),
            r#"/** @name Plum */
            :root { --background-primary: #201020; --text-normal: #f0e0f0;
                --brand-experiment: #c04080; --font-primary: serif; }"#,
        )
        .unwrap();
        let themes = Themes::read(folder.path(), &Layout::read(folder.path()));
        let plum = themes
            .css
            .iter()
            .find(|theme| theme.id == "css:plum")
            .expect("the CSS theme is listed");
        assert_eq!(plum.name, "Plum");
        let shown = themes.find("css:plum").expect("its colours");
        assert_eq!(shown.chat, crate::style::hex(0x201020));
        assert_eq!(shown.accent, crate::style::hex(0xc04080));
    }
}
