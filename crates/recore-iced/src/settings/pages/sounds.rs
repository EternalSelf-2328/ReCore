//! Sounds: the sound theme, each sound's own theme and volume, the interface
//! sounds, and a Play button for each.

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};
use recore_core::sounds::{InterfaceSound, Sound, SoundSet};

use super::{Pref, pref};
use crate::settings::{Drop, Event, Page};
use crate::{
    app::Message,
    controls::slider,
    form::{self},
    pictures::Icon,
};

/// Each of ReCore's call sounds, in `Sound::ALL`'s order.
pub(super) const SOUND_NAMES: [&str; 12] = [
    "Someone joins the call",
    "Someone leaves the call",
    "Muting",
    "Unmuting",
    "Deafening",
    "Undeafening",
    "A message",
    "A mention",
    "Someone starts watching a stream",
    "Someone stops watching a stream",
    "Someone starts streaming",
    "Someone stops streaming",
];

/// Each of ReCord's interface sounds, in `InterfaceSound::ALL`'s order.
pub(super) const INTERFACE_NAMES: [&str; 10] = [
    "Hover tick",
    "Select",
    "Back",
    "Open",
    "Close",
    "Navigate",
    "Toggle",
    "Send",
    "Typing tick",
    "Boot signal",
];

/// A dropdown's options: the sound theme the whole interface uses, then each
/// sound theme of its own.
fn theme_options() -> Vec<String> {
    let mut options = Vec::with_capacity(SoundSet::ALL.len() + 1);
    options.push("Theme default".to_owned());
    options.extend(SoundSet::ALL.iter().map(|set| set.label().to_owned()));
    options
}

/// The choice `set` is in [`theme_options`]; 0 is the default.
fn theme_index(set: Option<SoundSet>) -> usize {
    set.and_then(|set| SoundSet::ALL.iter().position(|known| *known == set))
        .map_or(0, |index| index + 1)
}

/// The message that opens or closes a sound theme dropdown.
fn theme_dropdown(drop: Drop) -> Message {
    Message::Settings(Event::Drop(drop))
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn sounds_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let voice = &self.cx.layout.voice;
        let set = SoundSet::ALL
            .iter()
            .position(|set| *set == voice.sound_set)
            .unwrap_or(0);
        let volume = u32::from(voice.sound_volume.unwrap_or(100));
        let interface = u32::from(self.cx.layout.prefs.interface_volume.unwrap_or(45));
        let sets: Vec<String> = SoundSet::ALL
            .iter()
            .map(|set| set.label().to_owned())
            .collect();
        let mut parts = vec![
            form::title(theme, "Sounds"),
            form::note(
                theme,
                "ReCore plays these as you and others join and leave a call, as you mute and deafen, as others start and stop streaming, as they start and stop watching your stream or the one you watch, and for messages that notify, except in the conversation in front of you. The sound theme also plays the interface's feedback. Every sound can take a theme of its own. They play on the speakers chosen for calls.",
            ),
            form::group([
                form::heading(theme, "SOUND THEME"),
                form::dropdown(
                    sets,
                    Some(set),
                    self.s.open == Some(Drop::SoundSet),
                    theme_dropdown(Drop::SoundSet),
                    |index| pref(Pref::SoundSet(index)),
                )
                .view(self.cx.look),
                form::note(
                    theme,
                    "The ReCord sets are the signal pack and fourteen themes of the theme's companion plugin: the terminal set, Kenney's clicks and ticks, and the Arcade, Sci-fi, Mechanical, Minimal, Glass, Cinematic, Studio, Soft, Rubber, Organic, Dreamy and Zen sets. ReCore's instruments stay for the sounds they synthesize.",
                ),
                slider("Volume", volume, 200, |value| {
                    pref(Pref::SoundVolume(value, false))
                })
                .on_release(pref(Pref::SoundVolume(volume, true)))
                .view(theme),
                self.flip(
                    "Adjust each sound",
                    "Set how loud each one is, as a share of the volume above. 0% turns a sound off.",
                    voice.adjust_each_sound,
                    Pref::AdjustEach,
                ),
            ]),
        ];

        let mut each = column![].spacing(10);
        for (index, (name, sound)) in SOUND_NAMES.iter().zip(Sound::ALL).enumerate() {
            let value = u32::from(voice.each_sound.get(&sound).copied().unwrap_or(100));
            let chosen = voice.sound_themes.get(&sound).copied();
            let controls = row![
                form::dropdown(
                    theme_options(),
                    Some(theme_index(chosen)),
                    self.s.open == Some(Drop::SoundTheme(index)),
                    theme_dropdown(Drop::SoundTheme(index)),
                    move |choice| pref(Pref::SoundTheme(index, choice)),
                )
                .width(230.0)
                .view(self.cx.look),
            ];
            let mut line = row![
                container(
                    text(*name)
                        .size(14)
                        .color(theme.text)
                        .wrapping(text::Wrapping::None)
                )
                .width(Length::Fill),
                controls,
            ]
            .spacing(12)
            .align_y(Alignment::Center);
            if voice.adjust_each_sound {
                line = line.push(
                    container(
                        slider(*name, value, 100, move |value| {
                            pref(Pref::EachSound(index, value, false))
                        })
                        .on_release(pref(Pref::EachSound(index, value, true)))
                        .view(theme),
                    )
                    .width(Length::Fill),
                );
            }
            line = line.push(
                self.cx
                    .look
                    .icon_button(Icon::Play, "Play", pref(Pref::PlaySound(index)))
                    .size(32.0, 18),
            );
            each = each.push(line);
        }
        parts.push(form::group([
            form::heading(theme, "EACH CALL SOUND"),
            form::note(
                theme,
                "How each call and message sound plays. Theme default follows the sound theme above; any other choice keeps this one sound on that theme.",
            ),
            form::buttons([crate::controls::action(
                theme,
                "Use the sound theme for every sound",
                crate::controls::Kind::Plain,
                Some(pref(Pref::ResetSoundThemes)),
            )
            .small()
            .into()]),
            each.into(),
        ]));

        let mut interface_rows = column![].spacing(10);
        for (index, (name, sound)) in INTERFACE_NAMES.iter().zip(InterfaceSound::ALL).enumerate() {
            let chosen = self.cx.layout.prefs.interface_themes.get(&sound).copied();
            interface_rows = interface_rows.push(
                row![
                    container(
                        text(*name)
                            .size(14)
                            .color(theme.text)
                            .wrapping(text::Wrapping::None)
                    )
                    .width(Length::Fill),
                    form::dropdown(
                        theme_options(),
                        Some(theme_index(chosen)),
                        self.s.open == Some(Drop::InterfaceTheme(index)),
                        theme_dropdown(Drop::InterfaceTheme(index)),
                        move |choice| pref(Pref::InterfaceTheme(index, choice)),
                    )
                    .width(230.0)
                    .view(self.cx.look),
                    self.cx
                        .look
                        .icon_button(Icon::Play, "Play", pref(Pref::PlayInterface(index)))
                        .size(32.0, 18),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        parts.push(form::group([
            form::heading(theme, "INTERFACE SOUNDS"),
            form::note(
                theme,
                "ReCord's own feedback: a tick on buttons and menu rows, a select on a click, a back on Escape, open and close for menus and dialogs, navigation for a room change, typing ticks and the boot signal. Theme default follows the sound theme above. These play on the speakers chosen for calls.",
            ),
            self.flip(
                "Interface sounds",
                "The master switch for every interface sound.",
                self.cx.layout.prefs.interface_sounds(),
                Pref::InterfaceSounds,
            ),
            slider("Interface volume", interface, 100, |value| {
                pref(Pref::InterfaceVolume(value, false))
            })
            .on_release(pref(Pref::InterfaceVolume(interface, true)))
            .view(theme),
            self.flip(
                "Hover ticks",
                "A short tick when the pointer comes onto a button, a menu row, a server, a channel or a member.",
                self.cx.layout.prefs.hover_sounds.unwrap_or(true),
                Pref::HoverSounds,
            ),
            self.flip(
                "Typing ticks",
                "A quiet tick for each character written in a message field.",
                self.cx.layout.prefs.typing_sounds.unwrap_or(true),
                Pref::TypingSounds,
            ),
            self.flip(
                "Boot signal",
                "The signal that plays with ReCore's boot screen.",
                self.cx.layout.prefs.boot_sound.unwrap_or(true),
                Pref::BootSound,
            ),
            form::heading(theme, "EACH INTERFACE SOUND"),
            interface_rows.into(),
        ]));
        form::page(parts)
    }
}
