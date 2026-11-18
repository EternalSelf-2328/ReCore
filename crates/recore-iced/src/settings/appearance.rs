//! Appearance: the themes as small windows drawn in their own colours (a
//! click shows one at once and keeps it), New theme and each custom theme's
//! pencil opening the editor, whose changes show at once (Save keeps them
//! among the person's own themes, Cancel puts the theme from before back),
//! and the profile switches. See docs/themes.md.

use std::fmt::Write as _;

use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, stack, text},
};
use recore_core::{
    application::Action,
    theme::{self as palette, Custom},
};
use recore_css::{OptionKind, ThemeOption};

use super::{Drop, Event as SettingsEvent, Page, Settings};
use crate::{
    app::Message,
    controls::{Kind, action, choices_within, lights, placed, slider},
    ctx::{Cx, Pref as Shown, Request},
    fonts::{BOLD, MEDIUM},
    form::{self, switch},
    pictures::Icon,
    themes::color,
};

/// A theme's tile: at least this wide, this tall, this far apart.
const TILE: f32 = 150.0;
const TILE_HEIGHT: f32 = 124.0;
const TILE_GAP: f32 = 12.0;
/// The editor's colours: two columns from this width, each row this tall.
const TWO_COLUMNS: f32 = 520.0;
const COLOUR_GAP: f32 = 24.0;
/// The name field.
const NAME: &str = "theme-name";

/// The theme editor while it's open: the theme as edited, each colour's
/// text and whether it's wrong, its name, and what it started from.
pub struct Editor {
    /// The custom theme edited, by id; none for a new one.
    pub editing: Option<String>,
    pub base: &'static str,
    pub theme: palette::Theme,
    pub name: String,
    /// Each colour's field, in `Theme::NAMES`' order: its name as
    /// `layout.json` has it, its text and whether it's a colour.
    pub fields: Vec<(String, String, bool)>,
    /// The theme a new one started as, among `palette::starts`.
    pub start: usize,
    /// Delete was pressed once; the next press deletes.
    pub confirming: bool,
}

impl Editor {
    fn new(
        editing: Option<String>,
        base: &'static str,
        theme: palette::Theme,
        name: String,
    ) -> Self {
        let mut editor = Self {
            editing,
            base,
            theme,
            name,
            fields: Vec::new(),
            start: 0,
            confirming: false,
        };
        editor.fill();
        editor
    }

    /// Every field's text from the theme's colours.
    fn fill(&mut self) {
        self.fields = palette::Theme::NAMES
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                Some((
                    palette::token(name),
                    palette::hex(self.theme.colour(index)?),
                    false,
                ))
            })
            .collect();
    }

    /// Takes a colour's text as typed: the theme takes it if it's a colour.
    /// Returns whether the theme changed.
    fn edit(&mut self, index: usize, text: String) -> bool {
        let parsed = palette::parse(&text);
        let Some(field) = self.fields.get_mut(index) else {
            return false;
        };
        (field.1, field.2) = (text, parsed.is_none());
        match (parsed, self.theme.colour_mut(index)) {
            (Some(colour), Some(slot)) if *slot != colour => {
                *slot = colour;
                true
            }
            _ => false,
        }
    }

    fn valid(&self) -> bool {
        self.fields.iter().all(|(_, _, wrong)| !wrong)
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    New,
    Edit(String),
    Start(usize),
    Colour(usize, String),
    Dark(bool),
    Name(String),
    Save,
    Cancel,
    Delete,
    Profile(Cosmetic, bool),
    Reload,
    OpenThemes,
    Accent(String),
    Preset(&'static str),
    ResetAccent,
    Effects(bool),
    LowPower(bool),
    Scanlines(u8),
    Grain(u8),
    Vignette(u8),
    Beam(bool),
    Flicker(bool),
    Frame(bool),
    AvatarFilter(bool),
    Background,
    ClearBackground,
    BackgroundDim(u8),
    PanelOpacity(u8),
    Font(&'static str),
    Motion(crate::themes::Motion),
    /// A CSS option while it's being typed, and when it's applied.
    CssDraft(String, String),
    CssApply(String, String),
    /// The picture button beside a CSS image option.
    CssImage(String),
    CssReset,
    /// Show every custom property, not only the readable ones.
    CssAll(bool),
}

/// What an Appearance tile is: one ReCore comes with, one the person made,
/// or a Discord CSS theme in a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    BuiltIn,
    Custom,
    Css,
}

/// Appearance's profile switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cosmetic {
    Effects,
    Decorations,
    Nameplates,
    Moving,
    NameStyles,
    ServerTags,
    Frames,
}

fn theme_message(event: Event) -> Message {
    Message::Settings(SettingsEvent::Theme(event))
}

impl Settings {
    pub(super) fn theme_event(&mut self, event: Event, cx: &mut Cx<'_>) {
        match event {
            Event::Choose(id) => {
                self.css_text.clear();
                choose(cx, &id);
            }
            Event::New => self.new_theme(cx),
            Event::Edit(id) => {
                let Some(custom) = cx.themes.custom.iter().find(|custom| custom.id == id) else {
                    return;
                };
                let editor = Editor::new(
                    Some(custom.id.clone()),
                    custom.base,
                    custom.theme,
                    custom.name.clone(),
                );
                self.open_editor(editor, cx);
            }
            Event::Start(index) => {
                let starts: Vec<_> = palette::starts(&cx.themes.custom)
                    .map(|start| (start.base, start.theme, copy_name(start.name)))
                    .collect();
                let Some(editor) = self.editor.as_mut() else {
                    return;
                };
                let Some((base, theme, name)) = starts.get(index).cloned() else {
                    return;
                };
                // It takes the new start's name too, unless the person
                // named it.
                if starts
                    .get(editor.start)
                    .is_some_and(|before| before.2 == editor.name)
                {
                    editor.name = name;
                }
                editor.base = base;
                editor.theme = theme;
                editor.start = index;
                editor.fill();
                self.open = None;
                self.show_edited(cx);
            }
            Event::Colour(index, text) => {
                let changed = self
                    .editor
                    .as_mut()
                    .is_some_and(|editor| editor.edit(index, text));
                if changed {
                    self.show_edited(cx);
                }
            }
            Event::Dark(dark) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.theme.dark = dark;
                }
                self.show_edited(cx);
            }
            Event::Name(name) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.name = name;
                }
            }
            Event::Save => self.save_theme(cx),
            Event::Cancel => self.cancel_theme(cx),
            Event::Delete => self.delete_theme(cx),
            Event::Profile(cosmetic, on) => self::cosmetic(cx, cosmetic, on),
            Event::Reload => {
                self.css_text.clear();
                cx.themes.reload();
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::OpenThemes => crate::themes::open_folder(cx.themes.themes_folder()),
            Event::Accent(text) => {
                self.accent = Some(text.clone());
                if let Some(accent) = palette::parse(&text) {
                    let root = cx.client.root().to_path_buf();
                    cx.themes.set_accent(&root, Some(accent));
                    cx.request(Request::Changed(Shown::Theme));
                }
            }
            Event::Preset(name) => {
                let (hex, theme) = preset(name);
                let accent = palette::parse(hex);
                let root = cx.client.root().to_path_buf();
                cx.themes.set_accent(&root, accent);
                self.accent = Some(hex.to_owned());
                if cx.themes.chosen_id() != theme {
                    cx.themes.choose(&root, theme);
                }
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::ResetAccent => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_accent(&root, None);
                self.accent = Some(palette::hex(palette::RECORE_DARK.accent));
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Effects(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_effects(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::LowPower(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_low_power(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Scanlines(percent) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_scanlines(&root, percent);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Grain(percent) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_grain(&root, percent);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Vignette(percent) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_vignette(&root, percent);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Beam(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_beam(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Flicker(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_flicker(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Frame(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_frame(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::AvatarFilter(on) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_avatar_filter(&root, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Background => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .pick_file()
                {
                    let root = cx.client.root().to_path_buf();
                    cx.themes.set_background(&root, Some(path));
                    cx.request(Request::Changed(Shown::Theme));
                }
            }
            Event::ClearBackground => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_background(&root, None);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::BackgroundDim(percent) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_background_dim(&root, percent);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::PanelOpacity(percent) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_panel_opacity(&root, percent);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Font(name) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_font(&root, name);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Motion(motion) => {
                let root = cx.client.root().to_path_buf();
                cx.themes.set_motion(&root, motion);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::CssDraft(name, value) => {
                self.css_text.insert(name.clone(), value.clone());
                // Colours show as they become valid; other values wait for
                // Enter or the slider's release, so a half-typed font or
                // path doesn't reload the theme on every key.
                let is_color = cx
                    .themes
                    .chosen_css()
                    .and_then(|css| css.options.iter().find(|option| option.name == name))
                    .is_some_and(|option| matches!(option.kind, OptionKind::Color { .. }));
                if is_color && recore_css::color::parse(&value, cx.theme.dark).is_some() {
                    self.css_apply(cx, &name, &value);
                }
            }
            Event::CssApply(name, value) => self.css_apply(cx, &name, &value),
            Event::CssImage(name) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .pick_file()
                {
                    let value = path.display().to_string();
                    self.css_text.insert(name.clone(), value.clone());
                    self.css_apply(cx, &name, &value);
                }
            }
            Event::CssReset => {
                let id = cx.themes.chosen_id().to_owned();
                let names: Vec<String> = cx
                    .themes
                    .chosen_css()
                    .map(|css| {
                        css.options
                            .iter()
                            .map(|option| option.name.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                self.css_text.retain(|name, _| !names.contains(name));
                let root = cx.client.root().to_path_buf();
                cx.themes.clear_options(&root, &id);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::CssAll(on) => self.show_all_css = on,
        }
    }

    /// Applies one of a CSS theme's options and keeps it for that theme.
    fn css_apply(&mut self, cx: &mut Cx<'_>, name: &str, value: &str) {
        let Some(option) = cx
            .themes
            .chosen_css()
            .and_then(|css| css.options.iter().find(|option| option.name == name))
            .cloned()
        else {
            return;
        };
        if matches!(option.kind, OptionKind::Color { .. })
            && recore_css::color::parse(value, cx.theme.dark).is_none()
        {
            return;
        }
        self.css_text.insert(name.to_owned(), value.to_owned());
        let applied = css_option_value(&option, value);
        let id = cx.themes.chosen_id().to_owned();
        let root = cx.client.root().to_path_buf();
        cx.themes.set_option(&root, &id, name, applied);
        cx.request(Request::Changed(Shown::Theme));
    }

    /// Opens the editor on a copy of the theme shown, which can start from
    /// any other.
    fn new_theme(&mut self, cx: &mut Cx<'_>) {
        let current = &cx.theme.id;
        let starts: Vec<_> = palette::starts(&cx.themes.custom)
            .map(|start| {
                (
                    start.id.to_owned(),
                    start.base,
                    start.theme,
                    copy_name(start.name),
                )
            })
            .collect();
        let start = starts
            .iter()
            .position(|(id, ..)| id == current)
            .unwrap_or(0);
        let (_, base, theme, name) = starts[start].clone();
        let mut editor = Editor::new(None, base, theme, name);
        editor.start = start;
        self.open_editor(editor, cx);
    }

    fn open_editor(&mut self, editor: Editor, cx: &mut Cx<'_>) {
        self.editor = Some(editor);
        self.show_edited(cx);
    }

    /// The window in the theme as edited, without keeping it.
    fn show_edited(&mut self, cx: &mut Cx<'_>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let id = editor.editing.as_deref().unwrap_or("editing");
        self.preview = Some(crate::themes::theme(id, &editor.theme));
        cx.request(Request::Changed(Shown::Theme));
    }

    /// Closes the editor: the window shows the theme chosen again.
    fn close_editor(&mut self, cx: &mut Cx<'_>) -> Option<Editor> {
        let editor = self.editor.take()?;
        if self.preview.take().is_some() {
            cx.request(Request::Changed(Shown::Theme));
        }
        Some(editor)
    }

    /// Keeps the theme as edited among the person's own, and chooses it.
    fn save_theme(&mut self, cx: &mut Cx<'_>) {
        if !self.editor.as_ref().is_some_and(Editor::valid) {
            return;
        }
        let Some(editor) = self.close_editor(cx) else {
            return;
        };
        let name = match editor.name.trim() {
            "" => "My theme".to_owned(),
            name => name.to_owned(),
        };
        let custom = &mut cx.themes.custom;
        let saved = editor
            .editing
            .as_ref()
            .and_then(|id| custom.iter_mut().find(|custom| custom.id == *id));
        let id = match saved {
            Some(saved) => {
                saved.name = name;
                saved.theme = editor.theme;
                saved.id.clone()
            }
            None => {
                let id = palette::new_id(custom);
                custom.push(Custom {
                    id: id.clone(),
                    name,
                    base: editor.base,
                    theme: editor.theme,
                });
                id
            }
        };
        cx.keep("custom_themes", &cx.themes.custom);
        choose(cx, &id);
    }

    /// Closes the editor and shows the theme from before it opened.
    pub(super) fn cancel_theme(&mut self, cx: &mut Cx<'_>) {
        self.close_editor(cx);
    }

    /// Deletes the theme edited; if it was the chosen one, Yap shows.
    fn delete_theme(&mut self, cx: &mut Cx<'_>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if !editor.confirming {
            editor.confirming = true;
            return;
        }
        let Some(editor) = self.close_editor(cx) else {
            return;
        };
        if let Some(id) = editor.editing {
            cx.themes.custom.retain(|custom| custom.id != id);
            cx.keep("custom_themes", &cx.themes.custom);
        }
        let (id, _) = palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
        let id = id.to_owned();
        choose(cx, &id);
    }
}

/// Chooses the theme `id` and keeps the choice, if there's such a theme:
/// the window shows it.
fn choose(cx: &mut Cx<'_>, id: &str) {
    let Some(theme) = cx.themes.find(id) else {
        return;
    };
    cx.themes.choose(cx.client.root(), &theme.id);
    cx.request(Request::Changed(Shown::Theme));
}

/// A profile switch: kept, and the core told what may now show.
fn cosmetic(cx: &mut Cx<'_>, cosmetic: Cosmetic, on: bool) {
    let prefs = &mut cx.layout.prefs;
    let (slot, key) = match cosmetic {
        Cosmetic::Effects => (&mut prefs.profile_effects, "profile_effects"),
        Cosmetic::Decorations => (&mut prefs.decorations, "decorations"),
        Cosmetic::Nameplates => (&mut prefs.nameplates, "nameplates"),
        Cosmetic::Moving => (&mut prefs.moving_cosmetics, "moving_cosmetics"),
        Cosmetic::NameStyles => (&mut prefs.name_styles, "name_styles"),
        Cosmetic::ServerTags => (&mut prefs.server_tags, "server_tags"),
        Cosmetic::Frames => (&mut prefs.profile_frames, "profile_frames"),
    };
    *slot = Some(on);
    cx.keep(key, on);
    if cosmetic == Cosmetic::Nameplates {
        cx.dispatch(Action::HideNameplates(!on));
    } else {
        let cosmetics = cx.layout.prefs.cosmetics();
        cx.dispatch(Action::SetCosmetics(cosmetics));
    }
    cx.request(Request::Changed(Shown::Roster));
}

impl<'a> Page<'a> {
    pub(super) fn appearance_page(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        if let Some(editor) = &settings.editor {
            return self.editor_view(editor, settings.open == Some(Drop::Start));
        }
        let tiles = self.theme_tiles();
        let prefs = &self.cx.layout.prefs;
        let cosmetics = prefs.cosmetics();
        let accent_text = settings.accent.as_deref().unwrap_or("#e94221");
        let accent_wrong = palette::parse(accent_text).is_none();
        let profile = |label, description, on: bool, which| {
            switch(label, on, Some(theme_message(Event::Profile(which, !on))))
                .description(description)
                .view(self.cx.look)
        };
        form::page([
            form::title(theme, "Appearance"),
            form::group([
                form::beside(
                    form::heading(theme, "THEME"),
                    row![
                        action(
                            theme,
                            "Reload CSS",
                            Kind::Plain,
                            Some(theme_message(Event::Reload))
                        )
                        .small(),
                        action(
                            theme,
                            "Themes folder",
                            Kind::Plain,
                            Some(theme_message(Event::OpenThemes))
                        )
                        .small(),
                        action(
                            theme,
                            "New theme",
                            Kind::Plain,
                            Some(theme_message(Event::New))
                        )
                        .small()
                        .icon(self.cx.look.icon(Icon::Plus, 16, crate::theme::rgb(theme.text))),
                    ]
                    .spacing(8),
                ),
                form::note(
                    theme,
                    css_note(self.cx.themes),
                ),
                tiles,
            ]),
            form::group([
                form::heading(theme, "RECORE ACCENT"),
                form::note(
                    theme,
                    "ReCore and ReCore Paper derive every accent shade from this colour. Other themes keep their own.",
                ),
                form::colour_row(
                    theme,
                    "Accent",
                    crate::themes::color(
                        self.cx.themes.accent.unwrap_or(palette::RECORE_DARK.accent),
                    ),
                    accent_text,
                    accent_wrong,
                    move |text| theme_message(Event::Accent(text)),
                ),
                row![
                    action(theme, "Amber", Kind::Plain, Some(theme_message(Event::Preset("amber")))).small(),
                    action(theme, "Phosphor", Kind::Plain, Some(theme_message(Event::Preset("phosphor")))).small(),
                    action(theme, "Ice", Kind::Plain, Some(theme_message(Event::Preset("ice")))).small(),
                    action(theme, "Paper", Kind::Plain, Some(theme_message(Event::Preset("paper")))).small(),
                    action(theme, "Terminal", Kind::Plain, Some(theme_message(Event::Preset("terminal")))).small(),
                    action(theme, "Reset", Kind::Plain, Some(theme_message(Event::ResetAccent))).small(),
                ]
                .spacing(8)
                .into(),
                switch(
                    "CRT layer",
                    self.cx.themes.effects,
                    Some(theme_message(Event::Effects(!self.cx.themes.effects))),
                )
                .description(
                    "Scanlines, film grain, the vignette and the corner brackets over the window.",
                )
                .view(self.cx.look),
                switch(
                    "Low power",
                    self.cx.themes.low_power,
                    Some(theme_message(Event::LowPower(!self.cx.themes.low_power))),
                )
                .description("Drops the grain and the constant repaints; the scanlines and vignette stay.")
                .view(self.cx.look),
            ]),
            form::group([
                form::heading(theme, "CRT DETAILS"),
                form::note(
                    theme,
                    "ReCord's own knobs. They apply to ReCore and ReCore Paper; a CSS theme keeps the values its file names.",
                ),
                switch(
                    "Beam",
                    self.cx.themes.recore.beam,
                    Some(theme_message(Event::Beam(!self.cx.themes.recore.beam))),
                )
                .description("The slow band travelling down the screen.")
                .view(self.cx.look),
                switch(
                    "Flicker",
                    self.cx.themes.recore.flicker,
                    Some(theme_message(Event::Flicker(!self.cx.themes.recore.flicker))),
                )
                .description("A quiet, irregular brightness change.")
                .view(self.cx.look),
                switch(
                    "Corner frame",
                    self.cx.themes.recore.frame > 0.0,
                    Some(theme_message(Event::Frame(self.cx.themes.recore.frame <= 0.0))),
                )
                .description("The four corner brackets around the window.")
                .view(self.cx.look),
                switch(
                    "Avatar scanlines",
                    self.cx.themes.recore.avatar_filter,
                    Some(theme_message(Event::AvatarFilter(
                        !self.cx.themes.recore.avatar_filter,
                    ))),
                )
                .description("ReCord's warm grade and fine lines over every picture.")
                .view(self.cx.look),
                slider(
                    "Scanlines",
                    (self.cx.themes.recore.scanlines * 100.0).round() as u32,
                    100,
                    |value| theme_message(Event::Scanlines(value as u8)),
                )
                .step(1)
                .view(self.cx.look.theme),
                slider(
                    "Film grain",
                    (self.cx.themes.recore.grain * 100.0).round() as u32,
                    100,
                    |value| theme_message(Event::Grain(value as u8)),
                )
                .step(1)
                .view(self.cx.look.theme),
                slider(
                    "Vignette",
                    (self.cx.themes.recore.vignette * 100.0).round() as u32,
                    100,
                    |value| theme_message(Event::Vignette(value as u8)),
                )
                .step(1)
                .view(self.cx.look.theme),
            ]),
            form::group([
                form::heading(theme, "RECORE WINDOW"),
                form::note(
                    theme,
                    "ReCord's surface picture and type. Lower the panel opacity to let the picture show through, as ReCord does.",
                ),
                row![
                    action(
                        theme,
                        "Choose background",
                        Kind::Plain,
                        Some(theme_message(Event::Background)),
                    )
                    .small(),
                    action(
                        theme,
                        "Clear",
                        Kind::Plain,
                        Some(theme_message(Event::ClearBackground)),
                    )
                    .small(),
                ]
                .spacing(8)
                .into(),
                form::note(
                    theme,
                    self.cx
                        .themes
                        .recore
                        .background
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "No background picture.".into()),
                ),
                slider(
                    "Background dim",
                    (self.cx.themes.recore.background_dim * 100.0).round() as u32,
                    100,
                    |value| theme_message(Event::BackgroundDim(value as u8)),
                )
                .step(1)
                .view(self.cx.look.theme),
                slider(
                    "Panel opacity",
                    (self.cx.themes.recore.panel_opacity * 100.0).round() as u32,
                    100,
                    |value| theme_message(Event::PanelOpacity(value as u8)),
                )
                .step(1)
                .view(self.cx.look.theme),
                form::heading(theme, "TYPE"),
                row![
                    action(theme, "Pixel", Kind::Plain, Some(theme_message(Event::Font("pixel")))).small(),
                    action(theme, "Pixel mono", Kind::Plain, Some(theme_message(Event::Font("mono")))).small(),
                    action(theme, "System", Kind::Plain, Some(theme_message(Event::Font("system")))).small(),
                ]
                .spacing(8)
                .into(),
                form::heading(theme, "MOTION"),
                row![
                    action(theme, "Full", Kind::Plain, Some(theme_message(Event::Motion(crate::themes::Motion::Full)))).small(),
                    action(theme, "Subtle", Kind::Plain, Some(theme_message(Event::Motion(crate::themes::Motion::Subtle)))).small(),
                    action(theme, "Off", Kind::Plain, Some(theme_message(Event::Motion(crate::themes::Motion::Off)))).small(),
                ]
                .spacing(8)
                .into(),
                form::note(
                    theme,
                    match self.cx.themes.motion() {
                        crate::themes::Motion::Full => "Everything: the boot signal, the stepped popups, the pulsing badges and the live presence blink.",
                        crate::themes::Motion::Subtle => "Menus and dialogs only; the boot signal stays, the loops hold still.",
                        crate::themes::Motion::Off => "No motion loops; the boot signal still finishes.",
                    },
                ),
            ]),
            self.css_options_view(),
            self.title_bar_switch(),
            column![
                form::heading(theme, "PROFILES"),
                profile(
                    "Play profile effects",
                    "The animations some people put over their profile cards.",
                    cosmetics.effects,
                    Cosmetic::Effects,
                ),
                profile(
                    "Show avatar decorations",
                    "The pictures around people's avatars.",
                    cosmetics.decorations,
                    Cosmetic::Decorations,
                ),
                profile(
                    "Show nameplates",
                    "The pictures Discord members equip behind their rows in the member list and on your account card.",
                    prefs.nameplates.unwrap_or(true),
                    Cosmetic::Nameplates,
                ),
                profile(
                    "Animate decorations and nameplates",
                    "They play when pointed at, as in Discord. Off, they hold still.",
                    cosmetics.moving,
                    Cosmetic::Moving,
                ),
                profile(
                    "Show name styles",
                    "The fonts and colours people give their display names.",
                    cosmetics.name_styles,
                    Cosmetic::NameStyles,
                ),
                profile(
                    "Show server tags",
                    "The tag and its badge beside people's names.",
                    cosmetics.server_tags,
                    Cosmetic::ServerTags,
                ),
                profile(
                    "Show profile frames",
                    "The frames some people put around their profile cards.",
                    cosmetics.frames,
                    Cosmetic::Frames,
                ),
            ]
            .spacing(12)
            .into(),
        ])
    }

    /// The themes as tiles, as many 150 px or wider across as fit.
    fn theme_tiles(self) -> Element<'a, Message> {
        iced::widget::responsive(move |size| {
            let columns = ((size.width + TILE_GAP) / (TILE + TILE_GAP))
                .floor()
                .max(1.0) as usize;
            let width = (size.width - (columns - 1) as f32 * TILE_GAP) / columns as f32;
            let cards: Vec<(String, String, Tile, palette::Theme)> = palette::BUILT_IN
                .iter()
                .map(|built_in| {
                    (
                        built_in.id.to_owned(),
                        built_in.name.to_owned(),
                        Tile::BuiltIn,
                        built_in.theme,
                    )
                })
                .chain(self.cx.themes.custom.iter().map(|custom| {
                    (
                        custom.id.clone(),
                        custom.name.clone(),
                        Tile::Custom,
                        custom.theme,
                    )
                }))
                .chain(self.cx.themes.css.iter().map(|css| {
                    (
                        css.id.clone(),
                        css.name.clone(),
                        Tile::Css,
                        crate::themes::css_palette(css),
                    )
                }))
                .collect();
            let mut rows = column![].spacing(TILE_GAP);
            for chunk in cards.chunks(columns) {
                let mut line = row![].spacing(TILE_GAP);
                for (id, name, kind, palette) in chunk {
                    line = line.push(self.theme_tile(id, name, *kind, palette, width));
                }
                rows = rows.push(line);
            }
            rows.into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// A theme to choose: a small window in its colours, its name under it,
    /// an accent ring and a tick on the one shown, and a pencil on the
    /// person's own.
    fn theme_tile(
        self,
        id: &str,
        name: &str,
        kind: Tile,
        palette: &palette::Theme,
        width: f32,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let current = self.cx.themes.chosen_id() == id;
        let preview = preview(palette, width - 16.0);
        let mut label = row![
            text(name.to_owned())
                .size(14)
                .font(if current { BOLD() } else { MEDIUM() })
                .color(if current { theme.text } else { theme.secondary })
                .wrapping(text::Wrapping::None)
                .width(Length::Fill)
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .height(24);
        if kind == Tile::Css {
            label = label.push(
                text("CSS")
                    .size(10)
                    .font(MEDIUM())
                    .color(theme.faint)
                    .wrapping(text::Wrapping::None),
            );
        }
        if current {
            label = label.push(self.cx.look.glyph(Icon::Check, 16, theme.accent));
        }
        let (accent, faint, line, hover) = (theme.accent, theme.faint, theme.line, theme.hover);
        let face = button(
            column![preview, container(label).padding([0, 2])]
                .spacing(6)
                .padding(8),
        )
        .padding(0)
        .width(width)
        .height(TILE_HEIGHT)
        .on_press(theme_message(Event::Choose(id.to_owned())))
        .style(move |_, status| {
            let lit = lights(status);
            button::Style {
                background: lit.then(|| hover.into()),
                border: Border {
                    color: if current {
                        accent
                    } else if lit {
                        faint
                    } else {
                        line
                    },
                    width: if current { 2.0 } else { 1.0 },
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        });
        if kind != Tile::Custom {
            return face.into();
        }
        let pencil = self
            .cx
            .look
            .icon_button(
                Icon::Pencil,
                "Edit",
                theme_message(Event::Edit(id.to_owned())),
            )
            .size(24.0, 14);
        stack![face, placed(pencil, width - 8.0 - 24.0, 8.0 + 78.0 + 6.0)].into()
    }

    /// The custom properties a CSS theme exposes, grouped, with a control
    /// for each. ReCore themes have their own accent and effects above.
    fn css_options_view(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let Some(css) = self.cx.themes.chosen_css() else {
            return Space::new().into();
        };
        let show_all = self.s.show_all_css;
        let mut groups: Vec<(&str, Vec<&ThemeOption>)> = Vec::new();
        for option in &css.options {
            if !show_all && option.group == "Other" {
                continue;
            }
            match groups.last_mut() {
                Some((group, options)) if *group == option.group => options.push(option),
                _ => groups.push((option.group.as_str(), vec![option])),
            }
        }
        let mut parts = vec![form::group([
            form::heading(theme, "THEME OPTIONS"),
            form::note(
                theme,
                format!(
                    "{} custom propert{} this theme exposes. Changes are kept for this theme.",
                    css.options.len(),
                    if css.options.len() == 1 { "y" } else { "ies" }
                ),
            ),
            switch(
                "Show every variable",
                show_all,
                Some(theme_message(Event::CssAll(!show_all))),
            )
            .description("Also lists the properties ReCore can't tell a use for, for a theme you're building.")
            .view(self.cx.look),
            action(
                theme,
                "Reset these options",
                Kind::Plain,
                Some(theme_message(Event::CssReset)),
            )
            .small()
            .into(),
        ])];
        for (group, options) in groups {
            let mut rows = column![].spacing(12);
            for option in options {
                rows = rows.push(self.css_control(option));
            }
            parts.push(form::group([
                form::heading(theme, group.to_uppercase()),
                rows.into(),
            ]));
        }
        column(parts).spacing(24).into()
    }

    /// One control for one CSS option.
    fn css_control(self, option: &'a ThemeOption) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let draft = self.s.css_text.get(&option.name).map(String::as_str);
        let shown = draft.unwrap_or_else(|| css_value_text(option));
        let description = option.description.as_deref();
        let label = option.label.as_str();
        let name = option.name.clone();
        let control: Element<'a, Message> = match &option.kind {
            OptionKind::Color { color } => form::colour_row(
                theme,
                label,
                Color::from_rgba8(color.r, color.g, color.b, f32::from(color.a) / 255.0),
                shown,
                recore_css::color::parse(shown, self.cx.look.theme.dark).is_none(),
                move |value| theme_message(Event::CssDraft(name.clone(), value)),
            ),
            OptionKind::Percent { value, percent } => slider(
                label,
                css_percent(shown)
                    .unwrap_or(*value)
                    .round()
                    .clamp(0.0, 100.0) as u32,
                100,
                {
                    let percent = *percent;
                    let name = name.clone();
                    move |value| {
                        theme_message(Event::CssDraft(
                            name.clone(),
                            css_percent_text(value, percent),
                        ))
                    }
                },
            )
            .on_release(theme_message(Event::CssApply(
                name.clone(),
                shown.to_owned(),
            )))
            .view(self.cx.look.theme),
            OptionKind::Toggle {
                on,
                on_value,
                off_value,
            } => switch(
                label,
                *on,
                Some(theme_message(Event::CssApply(
                    name.clone(),
                    if *on {
                        off_value.clone()
                    } else {
                        on_value.clone()
                    },
                ))),
            )
            .description(description.unwrap_or(""))
            .view(self.cx.look),
            OptionKind::Choice { choices } => {
                let current = choices
                    .iter()
                    .position(|choice| *choice == option.value)
                    .unwrap_or(0);
                let mut words = column![
                    text(label)
                        .size(14)
                        .color(theme.text)
                        .wrapping(text::Wrapping::None)
                ]
                .spacing(6);
                if let Some(description) = description {
                    words = words.push(form::note(theme, description));
                }
                words
                    .push(choices_within(theme, choices, current, 520.0, {
                        let name = name.clone();
                        move |index| {
                            theme_message(Event::CssApply(name.clone(), choices[index].to_owned()))
                        }
                    }))
                    .into()
            }
            OptionKind::Image { embedded, .. } => {
                let value = if *embedded { "" } else { shown };
                let submit = value.to_owned();
                let choose = name.clone();
                let clear = name.clone();
                let mut parts = vec![
                    container(
                        form::field(theme, option.name.clone(), label, value)
                            .on_input({
                                let name = name.clone();
                                move |value| theme_message(Event::CssDraft(name.clone(), value))
                            })
                            .on_submit(theme_message(Event::CssApply(name.clone(), submit))),
                    )
                    .width(Length::Fill)
                    .into(),
                    action(
                        theme,
                        "Choose…",
                        Kind::Plain,
                        Some(theme_message(Event::CssImage(choose))),
                    )
                    .small()
                    .into(),
                    action(
                        theme,
                        "Clear",
                        Kind::Plain,
                        Some(theme_message(Event::CssApply(clear, String::new()))),
                    )
                    .small()
                    .into(),
                ];
                if *embedded {
                    parts.insert(
                        0,
                        form::note(theme, "The theme carries this picture inside its file."),
                    );
                }
                let mut lines =
                    column![row(parts).spacing(8).align_y(Alignment::Center)].spacing(6);
                if let Some(description) = description {
                    lines = lines.push(form::note(theme, description));
                }
                lines.into()
            }
            OptionKind::Font | OptionKind::Text | OptionKind::Number { .. } => {
                let submit = shown.to_owned();
                container(
                    form::field(theme, option.name.clone(), label, shown)
                        .on_input({
                            let name = name.clone();
                            move |value| theme_message(Event::CssDraft(name.clone(), value))
                        })
                        .on_submit(theme_message(Event::CssApply(name.clone(), submit))),
                )
                .width(Length::Fill)
                .into()
            }
        };
        if description.is_some()
            && !matches!(
                option.kind,
                OptionKind::Toggle { .. } | OptionKind::Choice { .. } | OptionKind::Image { .. }
            )
        {
            column![control, form::note(theme, description.unwrap_or(""))]
                .spacing(4)
                .into()
        } else {
            control
        }
    }

    /// The editor: the name with Save and Cancel, what a new theme starts
    /// from, Dark, each colour's field, and Delete for a saved one.
    fn editor_view(self, editor: &'a Editor, starts_open: bool) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut parts = vec![
            form::title(theme, "Appearance"),
            form::group([
                form::heading(
                    theme,
                    if editor.editing.is_some() {
                        "EDIT THEME"
                    } else {
                        "NEW THEME"
                    },
                ),
                row![
                    form::field(theme, NAME, "Name", &editor.name)
                        .on_input(|name| theme_message(Event::Name(name)))
                        .on_submit(theme_message(Event::Save)),
                    action(
                        theme,
                        "Save",
                        Kind::Primary,
                        editor.valid().then(|| theme_message(Event::Save))
                    ),
                    action(
                        theme,
                        "Cancel",
                        Kind::Plain,
                        Some(theme_message(Event::Cancel))
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ]),
        ];
        if editor.editing.is_none() {
            let names: Vec<String> = palette::starts(&self.cx.themes.custom)
                .map(|start| start.name.to_owned())
                .collect();
            parts.push(form::group([
                form::heading(theme, "START FROM"),
                form::dropdown(
                    names,
                    Some(editor.start),
                    starts_open,
                    Message::Settings(SettingsEvent::Drop(Drop::Start)),
                    |index| theme_message(Event::Start(index)),
                )
                .view(self.cx.look),
            ]));
        }
        parts.push(
            switch(
                "Dark",
                editor.theme.dark,
                Some(theme_message(Event::Dark(!editor.theme.dark))),
            )
            .description("Drop-down lists and scroll bars draw dark to match.")
            .view(self.cx.look),
        );
        let colours = iced::widget::responsive(move |size| {
            let columns = if size.width >= TWO_COLUMNS { 2 } else { 1 };
            let per = editor.fields.len().div_ceil(columns);
            let mut line = row![].spacing(COLOUR_GAP);
            for column_index in 0..columns {
                let mut cells = column![];
                for index in column_index * per..((column_index + 1) * per).min(editor.fields.len())
                {
                    let (name, hex, wrong) = &editor.fields[index];
                    let shown = editor.theme.colour(index).map_or(Color::TRANSPARENT, color);
                    cells = cells.push(form::colour_row(
                        theme,
                        name,
                        shown,
                        hex,
                        *wrong,
                        move |text| theme_message(Event::Colour(index, text)),
                    ));
                }
                line = line.push(container(cells).width(Length::FillPortion(1)));
            }
            line.into()
        })
        .height(Length::Shrink);
        parts.push(form::group([
            form::heading(theme, "COLOURS"),
            form::note(
                theme,
                "Each as #rgb, #rrggbb or #rrggbbaa. The window shows a change as you type it.",
            ),
            colours.into(),
        ]));
        if editor.editing.is_some() {
            parts.push(form::buttons([action(
                theme,
                if editor.confirming {
                    "Delete it for good"
                } else {
                    "Delete theme"
                },
                Kind::Danger,
                Some(theme_message(Event::Delete)),
            )
            .small()
            .icon(
                self.cx
                    .look
                    .icon(Icon::Trash, 16, crate::theme::rgb(theme.on_danger)),
            )
            .into()]));
        }
        form::page(parts)
    }
}

/// What a theme copied from another is called until it's named.
fn copy_name(name: &str) -> String {
    format!("{name} copy")
}

/// The text a CSS option's field shows.
fn css_value_text(option: &ThemeOption) -> &str {
    match &option.kind {
        OptionKind::Image { url, embedded } => {
            if *embedded {
                ""
            } else {
                url.as_deref().unwrap_or("")
            }
        }
        _ => option.value.as_str(),
    }
}

/// `25%` or `0.25` as the 0..100 a slider shows.
fn css_percent(text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(percent) = text.strip_suffix('%') {
        return percent.trim().parse::<f32>().ok();
    }
    text.parse::<f32>().ok().map(|value| value * 100.0)
}

/// A slider's 0..100 as the theme writes it: `25%` or `0.25`.
fn css_percent_text(value: u32, percent: bool) -> String {
    if percent {
        format!("{value}%")
    } else {
        format!("{:.2}", value as f32 / 100.0)
    }
}

/// The CSS value an option is set to from the value its field or slider
/// shows.
fn css_option_value(option: &ThemeOption, value: &str) -> String {
    let value = value.trim();
    match &option.kind {
        OptionKind::Image { .. } => {
            if value.is_empty() || value.eq_ignore_ascii_case("none") {
                "none".into()
            } else if value.starts_with("url(") {
                value.to_owned()
            } else {
                format!("url(\"{}\")", value.replace('\\', "/").replace('"', "\\\""))
            }
        }
        OptionKind::Number { unit, .. } if !unit.is_empty() && value.parse::<f32>().is_ok() => {
            format!("{value}{unit}")
        }
        _ => value.to_owned(),
    }
}

/// ReCord's one-click palettes: the accent and the ReCore theme they pair
/// with. `terminal` uses ReCord's custom colours in the theme; ReCore keeps
/// the accent and the dark palette here.
fn preset(name: &str) -> (&'static str, &'static str) {
    match name {
        "phosphor" => ("#3ddc97", "recore"),
        "ice" => ("#4fb8c9", "recore"),
        "paper" => ("#b8401f", "recore-light"),
        "terminal" => ("#3ddc97", "recore"),
        _ => ("#e94221", "recore"),
    }
}

/// What Appearance says of the CSS themes it read, and where to put more.
fn css_note(themes: &crate::themes::Themes) -> String {
    let folder = themes.themes_folder().display();
    let mut note = if themes.css.is_empty() {
        format!(
            "No CSS themes yet. Put a Vencord, Vesktop or Discord .theme.css file in {folder}, then Reload CSS."
        )
    } else {
        format!(
            "{} CSS theme{} read from {folder} and the theme folders of Vencord, Vesktop and their kin. A CSS theme gives its colours, accent, font, radius and background; Discord's web-only rules have no nodes in ReCore and are not painted.",
            themes.css.len(),
            if themes.css.len() == 1 { "" } else { "s" }
        )
    };
    if !themes.problems.is_empty() {
        let _ = write!(
            note,
            " {} file{} could not be read.",
            themes.problems.len(),
            if themes.problems.len() == 1 { "" } else { "s" }
        );
    }
    note
}

/// A theme's small window, 78 px tall: the rail with three servers, the
/// sidebar's channels, two messages and the composer, in its colours.
fn preview<'a>(palette: &palette::Theme, width: f32) -> Element<'a, Message> {
    let c = |colour| color(colour);
    let (rail, sidebar, chat, raised, accent, ink, muted) = (
        c(palette.rail),
        c(palette.sidebar),
        c(palette.chat),
        c(palette.raised),
        c(palette.accent),
        c(palette.text),
        c(palette.muted),
    );
    let blob = |w: f32, h: f32, colour: Color, radius: f32| -> Element<'a, Message> {
        container(Space::new())
            .width(w)
            .height(h)
            .style(move |_| crate::style::rounded(colour, radius))
            .into()
    };
    let mut layers = stack![blob(width, 78.0, chat, 6.0)];
    layers = layers.push(blob(16.0, 78.0, rail, 0.0));
    for index in 0..3 {
        let (colour, radius) = if index == 0 {
            (accent, 3.0)
        } else {
            (raised, 5.0)
        };
        layers = layers.push(placed(
            blob(10.0, 10.0, colour, radius),
            3.0,
            6.0 + index as f32 * 13.0,
        ));
    }
    layers = layers.push(placed(blob(38.0, 78.0, sidebar, 0.0), 16.0, 0.0));
    for index in 0..4 {
        let w = if index == 1 {
            24.0
        } else {
            18.0 + (index % 2) as f32 * 4.0
        };
        let colour = if index == 1 { ink } else { muted };
        layers = layers.push(placed(
            blob(w, 4.0, colour, 2.0),
            22.0,
            8.0 + index as f32 * 11.0,
        ));
    }
    let room = width - 66.0;
    for index in 0..2 {
        let y = 8.0 + index as f32 * 22.0;
        let first = index == 0;
        layers = layers
            .push(placed(
                blob(10.0, 10.0, if first { accent } else { muted }, 5.0),
                60.0,
                y,
            ))
            .push(placed(
                blob(if first { 22.0 } else { 16.0 }, 4.0, ink, 2.0),
                74.0,
                y + 1.0,
            ))
            .push(placed(
                blob(
                    (room - 14.0).min(if first { 52.0 } else { 40.0 }),
                    3.0,
                    muted,
                    1.5,
                ),
                74.0,
                y + 8.0,
            ));
    }
    layers = layers
        .push(placed(blob(room, 12.0, raised, 4.0), 60.0, 78.0 - 18.0))
        .push(placed(
            blob(6.0, 6.0, accent, 3.0),
            60.0 + room - 10.0,
            78.0 - 18.0,
        ));
    container(layers.width(width).height(78))
        .clip(true)
        .style(|_| iced::widget::container::Style {
            border: crate::style::round(6),
            ..Default::default()
        })
        .into()
}

impl<'a> Page<'a> {
    /// Appearance's "Use the system title bar", where ReCore can draw its
    /// own: the desktop's way until it's chosen.
    pub(super) fn title_bar_switch(self) -> Element<'a, Message> {
        if !crate::window_bar::offered() {
            return Space::new().into();
        }
        let system = !self.cx.own_title_bar;
        switch(
            "Use the system title bar",
            system,
            Some(Message::Settings(SettingsEvent::Pref(
                super::pages::Pref::SystemTitleBar(!system),
            ))),
        )
        .description("Your desktop draws the window's title bar and buttons, as on its other windows. Off, ReCore draws its own.")
        .view(self.cx.look)
    }
}
