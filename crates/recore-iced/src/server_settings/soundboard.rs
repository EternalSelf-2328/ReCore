//! Soundboard: the server's sounds and their slots, each renamed or
//! deleted at once, and Add Sound, which chooses a file and names it in a
//! dialog.

use iced::{
    Alignment, Element, Length,
    widget::{column, row, text},
};
use recore_core::application::{ServerSettings, SoundboardInput, SoundboardPage};

use super::{Event, Screen, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    dialog, form,
};

/// The width of the sound dialog.
const DIALOG: f32 = 440.0;
/// The name field's id in the dialog.
pub(super) const NAME: &str = "sound-name";

impl<'a> View<'a> {
    /// Soundboard: how many sounds the server has and may have, Add Sound,
    /// and each sound with who added it and its buttons.
    pub(super) fn soundboard_page(
        self,
        _core: &'a ServerSettings,
        page: &'a SoundboardPage,
        _screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let saving = page.draft.as_ref().is_some_and(|draft| draft.saving);
        let may_add = !page.full() && !saving;
        let mut head = row![
            column![
                form::title(theme, "Soundboard"),
                form::note(
                    theme,
                    "Add sounds everyone in your server can play in voice channels.",
                ),
            ]
            .spacing(form::TIGHT)
            .width(Length::Fill),
            action(
                theme,
                "Add Sound",
                Kind::Primary,
                may_add.then(|| message(Event::PickSound)),
            ),
        ]
        .spacing(form::GAP)
        .align_y(Alignment::Center);
        head = head.push(
            text(format!("{} of {} sounds", page.sounds.len(), page.slots))
                .size(14)
                .color(theme.muted),
        );
        let mut list = column![].spacing(6).width(Length::Fill);
        if page.sounds.is_empty() {
            list = list.push(form::note(theme, "This server has no sounds yet."));
        }
        for sound in &page.sounds {
            let sound_id = sound.id.clone();
            let mut words = column![
                text(sound.name.as_str())
                    .size(15)
                    .color(if sound.available {
                        theme.text
                    } else {
                        theme.muted
                    }),
            ]
            .spacing(2);
            let mut under = Vec::new();
            if let Some(uploader) = &sound.uploader {
                under.push(uploader.clone());
            }
            if let Some(emoji) = &sound.emoji {
                under.push(emoji.clone());
            }
            if !sound.available {
                under.push("Unavailable without the boosts it takes".into());
            }
            if !under.is_empty() {
                words = words.push(text(under.join(" · ")).size(13).color(theme.muted));
            }
            let buttons = row![
                action(
                    theme,
                    "Rename",
                    Kind::Plain,
                    (!saving).then(|| {
                        message(Event::Soundboard(SoundboardInput::Edit(sound_id.clone())))
                    }),
                ),
                action(
                    theme,
                    "Delete",
                    Kind::Danger,
                    (!saving).then(|| {
                        message(Event::Soundboard(SoundboardInput::Delete(sound_id.clone())))
                    }),
                ),
            ]
            .spacing(form::TIGHT);
            let ground = theme.wash(0.03);
            list = list.push(
                iced::widget::container(
                    row![
                        column![text(sound.emoji.as_deref().unwrap_or("🔊")).size(20), words]
                            .spacing(6)
                            .width(Length::Fill),
                        buttons,
                    ]
                    .spacing(form::GAP)
                    .align_y(Alignment::Center),
                )
                .padding(10)
                .width(Length::Fill)
                .style(move |_| crate::style::rounded(ground, 8)),
            );
        }
        column![head, form::rule(theme), list]
            .spacing(form::GAP)
            .width(Length::Fill)
            .into()
    }

    /// The dialog that names a new sound, or renames one.
    pub(super) fn soundboard_dialog(
        self,
        page: &'a SoundboardPage,
        _screen: &'a Screen,
    ) -> Option<Element<'a, Message>> {
        let draft = page.draft.as_ref()?;
        let theme = self.cx.look.theme;
        let editing = draft.editing.is_some();
        let ready = draft.ready();
        let mut body = column![
            text(if editing { "Rename Sound" } else { "Add Sound" })
                .size(20)
                .font(crate::fonts::BOLD())
                .color(theme.text),
            form::entry(theme, NAME, "Sound name", draft.name.as_str())
                .on_input(|text| { message(Event::Soundboard(SoundboardInput::Name(text))) }),
        ]
        .spacing(14)
        .padding(16);
        if let Some(file) = &draft.file {
            body = body.push(form::note(
                theme,
                format!(
                    "{} · {}",
                    file.file_name,
                    recore_core::chat::file_size(file.data.len() as u64)
                ),
            ));
        }
        if let Some(error) = &draft.error {
            body = body.push(form::error(theme, error.as_str()));
        }
        body = body.push(
            row![
                iced::widget::Space::new().width(Length::Fill),
                action(
                    theme,
                    "Cancel",
                    Kind::Ghost,
                    (!draft.saving)
                        .then(|| { message(Event::Soundboard(SoundboardInput::Cancel)) }),
                ),
                action(
                    theme,
                    if draft.saving { "Saving..." } else { "Save" },
                    Kind::Primary,
                    ready.then(|| message(Event::Soundboard(SoundboardInput::Save))),
                ),
            ]
            .spacing(form::TIGHT),
        );
        Some(dialog::panel(
            theme,
            body.into(),
            DIALOG,
            None,
            message(Event::Soundboard(SoundboardInput::Cancel)),
        ))
    }
}

/// The id of the dialog's field, for the window to focus.
pub(super) fn dialog_field(page: &SoundboardPage) -> Option<&'static str> {
    page.draft.as_ref().map(|_| NAME)
}
