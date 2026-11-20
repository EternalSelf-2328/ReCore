//! Messages: how messages show, the clock and the date.

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

use super::{ANIMATES, Pref, message_size, pref};
use crate::settings::{Drop, Event as SettingsEvent, Page};
use crate::{
    app::Message,
    autocorrect::{self, Status},
    controls::{Kind, action, choices},
    fonts::BOLD,
    form::{self, switch},
};

pub(super) const DATE_FORMATS: [&str; 3] = ["Day/Month/Year", "Month/Day/Year", "Year-Month-Day"];

impl<'a> Page<'a> {
    pub(in crate::settings) fn messages_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let size = message_size(prefs.text_size);
        let twelve = prefs.twelve_hour_clock;
        let clock = format!(
            "{}{}{}",
            if twelve { "9:41" } else { "21:41" },
            if prefs.clock_seconds { ":05" } else { "" },
            if twelve { " PM" } else { "" }
        );
        let edge = theme.line;
        let preview = container(
            row![
                container(self.cx.avatar(None, "Sam", 40.0, 20.0, 1.0)).align_top(Length::Shrink),
                column![
                    row![
                        text("Sam").size(size).font(BOLD()).color(theme.text),
                        text(clock).size(size - 3.0).color(theme.muted)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text("Thinking co-op after nine, if people are up for it.")
                        .size(size)
                        .color(theme.text)
                ]
                .spacing(3)
                .width(Length::Fill)
            ]
            .spacing(16),
        )
        .padding(16)
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            border: crate::style::edge(edge, 8.0),
            ..Default::default()
        });
        let text_size = prefs.text_size.unwrap_or(1).clamp(0, 4) as usize;
        let date = prefs
            .date_format
            .and_then(|index| usize::try_from(index).ok())
            .unwrap_or(usize::from(twelve));
        let animate = ANIMATES
            .iter()
            .position(|kind| *kind == prefs.animate)
            .unwrap_or(0);
        let networks = if cfg!(feature = "matrix") && self.linked() {
            "Discord and Matrix show"
        } else if cfg!(feature = "matrix") {
            "Matrix shows"
        } else {
            "Discord shows"
        };
        form::page([
            form::title(theme, "Messages"),
            preview.into(),
            form::group([
                form::heading(theme, "FONT SIZE"),
                choices(
                    theme,
                    &["Small", "Default", "Large", "Larger", "Huge"],
                    text_size,
                    |index| pref(Pref::TextSize(index)),
                ),
                form::note(
                    theme,
                    "The messages' own type size, in the timeline and the composer. The interface size in Appearance grows every label and picture with it.",
                ),
            ]),
            form::group([
                form::heading(theme, "CLOCK"),
                choices(theme, &["24-hour", "12-hour"], usize::from(twelve), |index| {
                    pref(Pref::TwelveHour(index == 1))
                }),
                switch(
                    "Show seconds",
                    prefs.clock_seconds,
                    Some(pref(Pref::Seconds(!prefs.clock_seconds))),
                )
                .description("Message times show them, as 21:41:05.")
                .view(self.cx.look),
                form::heading(theme, "DATE FORMAT"),
                choices(theme, &DATE_FORMATS, date, |index| {
                    pref(Pref::DateFormat(index))
                }),
            ]),
            form::group([
                form::heading(theme, "PLAY GIFS, EMOJI AND STICKERS"),
                choices(
                    theme,
                    &["While ReCore is focused", "Always", "Never"],
                    animate,
                    |index| pref(Pref::Animate(index)),
                ),
            ]),
            form::group([
                form::heading(theme, "WRITING"),
                switch(
                    "Autocorrect while writing",
                    prefs.word_suggestions.unwrap_or(true),
                    Some(pref(Pref::WordSuggestions(
                        !prefs.word_suggestions.unwrap_or(true),
                    ))),
                )
                .description(
                    "A word missing its accents, or a common typo once the word is finished, is offered under the composer. Tab or a press takes it; nothing changes on its own.",
                )
                .view(self.cx.look),
            ]),
            form::group([
                form::heading(theme, "LANGUAGE YOU WRITE IN"),
                form::dropdown(
                    std::iter::once("Detect automatically".to_owned())
                        .chain(
                            crate::translate::LANGUAGES
                                .iter()
                                .map(|(_, name)| (*name).to_owned()),
                        )
                        .collect(),
                    Some(
                        prefs
                            .translate_source
                            .as_deref()
                            .and_then(|source| {
                                crate::translate::LANGUAGES
                                    .iter()
                                    .position(|(code, _)| code.eq_ignore_ascii_case(source))
                            })
                            .map_or(0, |index| index + 1),
                    ),
                    self.s.open == Some(Drop::TranslateSource),
                    Message::Settings(SettingsEvent::Drop(Drop::TranslateSource)),
                    |index| pref(Pref::TranslateSource(index)),
                )
                .view(self.cx.look),
                form::note(
                    theme,
                    "The language your messages are written in, and the one autocorrect uses. Detect automatically lets Google tell it for each message; autocorrect takes the language from the desktop.",
                ),
            ]),
            self.autocorrect_model_group(),
            form::group([
                form::heading(theme, "LANGUAGE TO SEND IN"),
                form::dropdown(
                    crate::translate::LANGUAGES
                        .iter()
                        .map(|(_, name)| (*name).to_owned())
                        .collect(),
                    Some(
                        prefs
                            .translate_target
                            .as_deref()
                            .and_then(|target| {
                                crate::translate::LANGUAGES
                                    .iter()
                                    .position(|(code, _)| code.eq_ignore_ascii_case(target))
                            })
                            .unwrap_or(1),
                    ),
                    self.s.open == Some(Drop::TranslateTarget),
                    Message::Settings(SettingsEvent::Drop(Drop::TranslateTarget)),
                    |index| pref(Pref::TranslateTarget(index)),
                )
                .view(self.cx.look),
                switch(
                    "Translate messages before sending",
                    prefs.translate_send.unwrap_or(false),
                    Some(pref(Pref::TranslateSend(!prefs.translate_send.unwrap_or(false)))),
                )
                .description(
                    "Each message goes to Google Translate from the language above into this one before it's sent.",
                )
                .view(self.cx.look),
                form::note(
                    theme,
                    "A message's menu → Translate still sends its text to Google Translate once, when asked, and shows the answer in ReCore.",
                ),
            ]),
            form::rule(theme),
            self.flip(
                "Show link previews",
                "Cards for links, videos and bots under messages. Linked pictures still show.",
                prefs.link_previews.unwrap_or(true),
                Pref::LinkPreviews,
            ),
            self.flip(
                "Collapse long messages",
                "Long messages and code show their first lines, with Show more to see the rest.",
                self.cx.layout.collapse_long,
                Pref::CollapseLong,
            ),
            self.flip(
                "Show boost gems",
                "A gem by the name of each member boosting their Discord server, in the member list.",
                prefs.boost_gems.unwrap_or(true),
                Pref::BoostGems,
            ),
            switch(
                "Let others see when you're typing",
                prefs.share_typing.unwrap_or(true),
                Some(pref(Pref::ShareTyping(!prefs.share_typing.unwrap_or(true)))),
            )
            .description(format!(
                "{networks} \"typing...\" while you write. Others still show here when they type."
            ))
            .view(self.cx.look),
            self.flip(
                "Plain folder icons",
                "A folder shows a folder, not the faces of its first four servers.",
                prefs.plain_folders.unwrap_or(false),
                Pref::PlainFolders,
            ),
            self.flip(
                "Open GIFs in Favorites",
                "The GIF picker starts on the GIFs you starred, rather than its home.",
                prefs.gif_favorites_first.unwrap_or(false),
                Pref::GifFavoritesFirst,
            ),
            self.flip(
                "Mention people you reply to",
                "They get notified of your reply. Switch it for one reply with @ On on the reply bar.",
                prefs.reply_ping.unwrap_or(true),
                Pref::ReplyPing,
            ),
            self.flip(
                "Remove tracking from links you send",
                "Takes out tags like utm_source and fbclid, and share ids from YouTube, Spotify and X. The links still go to the same page.",
                prefs.strip_tracking.unwrap_or(true),
                Pref::StripTracking,
            ),
        ])
    }

    /// The optional word list that corrects typos and accents in the
    /// language written in. Spanish accents work without it.
    fn autocorrect_model_group(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let autocorrect = self.cx.autocorrect;
        let language = autocorrect.language_name();
        let mut parts = vec![
            form::heading(theme, "AUTOCORRECT MODEL"),
            form::note(
                theme,
                "Word frequencies from FrequencyWords (CC BY-SA 4.0), built from OpenSubtitles. The list is kept on this device; nothing is sent while you type.",
            ),
        ];
        match &autocorrect.status {
            Status::Missing => {
                let fallback = if autocorrect.language.starts_with("es") {
                    "Without it, autocorrect only offers Spanish accents."
                } else {
                    "Without it, autocorrect offers nothing."
                };
                parts.push(form::note(
                    theme,
                    format!("The {language} model isn't downloaded. {fallback}"),
                ));
                parts.push(
                    action(
                        theme,
                        format!("Download {language} model"),
                        Kind::Plain,
                        Some(Message::Autocorrect(autocorrect::Event::Download)),
                    )
                    .into(),
                );
            }
            Status::Unsupported => parts.push(form::note(
                theme,
                "The word lists have no model for the language you write in, so autocorrect offers nothing in it.",
            )),
            Status::Downloading => parts.push(form::note(
                theme,
                format!("Downloading the {language} model..."),
            )),
            Status::Ready { words } => parts.push(form::beside(
                text(format!("{words} words kept on this device."))
                    .size(14)
                    .color(theme.secondary),
                action(
                    theme,
                    "Remove",
                    Kind::Plain,
                    Some(Message::Autocorrect(autocorrect::Event::Remove)),
                ),
            )),
            Status::Failed(error) => {
                parts.push(form::note_in(error.clone(), theme.danger));
                parts.push(
                    action(
                        theme,
                        "Try again",
                        Kind::Plain,
                        Some(Message::Autocorrect(autocorrect::Event::Download)),
                    )
                    .into(),
                );
            }
        }
        form::group(parts)
    }
}
