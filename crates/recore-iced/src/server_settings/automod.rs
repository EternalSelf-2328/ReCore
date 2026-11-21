//! AutoMod: a server's rules, each switched on or off or deleted at once,
//! and Add Rule, which makes a keyword rule with the actions it takes.

use iced::{
    Alignment, Element, Length,
    widget::{column, row, text},
};
use recore_core::application::{AutoModInput, AutoModPage, ServerSettings};

use super::{Drop, Event, Screen, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    dialog, form,
};

const DIALOG: f32 = 480.0;
pub(super) const NAME: &str = "automod-name";

impl<'a> View<'a> {
    /// AutoMod: Add Rule, then each rule with its trigger, its actions and
    /// its switch.
    pub(super) fn automod_page(
        self,
        _core: &'a ServerSettings,
        page: &'a AutoModPage,
        _screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let saving = page.draft.as_ref().is_some_and(|draft| draft.saving);
        let mut list = column![].spacing(6).width(Length::Fill);
        if page.rules.is_empty() {
            list = list.push(form::note(theme, "This server has no AutoMod rules."));
        }
        for rule in &page.rules {
            let trigger = match rule.trigger {
                1 => "Keyword".to_owned(),
                3 => "Spam".to_owned(),
                4 => "Preset words".to_owned(),
                5 => "Mention spam".to_owned(),
                6 => "Member profile".to_owned(),
                other => format!("Trigger {other}"),
            };
            let mut under = vec![trigger];
            if !rule.keywords.is_empty() {
                under.push(rule.keywords.join(", "));
            }
            if !rule.presets.is_empty() {
                let presets: Vec<&str> = rule
                    .presets
                    .iter()
                    .filter_map(|preset| match preset {
                        1 => Some("Profanity"),
                        2 => Some("Sexual content"),
                        3 => Some("Slurs"),
                        _ => None,
                    })
                    .collect();
                if !presets.is_empty() {
                    under.push(presets.join(", "));
                }
            }
            let mut acts = Vec::new();
            if rule.actions.contains(&1) {
                acts.push("Block message");
            }
            if rule.actions.contains(&2) {
                acts.push("Send alert");
            }
            if rule.actions.contains(&3) {
                acts.push("Time out member");
            }
            if !acts.is_empty() {
                under.push(acts.join(", "));
            }
            let id = rule.id.clone();
            let enabled = rule.enabled;
            let ground = theme.wash(0.03);
            list = list.push(
                iced::widget::container(
                    row![
                        column![
                            text(rule.name.as_str()).size(15).color(theme.text),
                            text(under.join(" · ")).size(13).color(theme.muted),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        form::switch(
                            "Enabled",
                            enabled,
                            (!saving).then(|| {
                                message(Event::AutoMod(AutoModInput::SetEnabled {
                                    id: id.clone(),
                                    enabled: !enabled,
                                }))
                            }),
                        )
                        .view(self.cx.look),
                        action(
                            theme,
                            "Delete",
                            Kind::Danger,
                            (!saving)
                                .then(|| message(Event::AutoMod(AutoModInput::Delete(id.clone())))),
                        ),
                    ]
                    .spacing(form::GAP)
                    .align_y(Alignment::Center),
                )
                .padding(10)
                .width(Length::Fill)
                .style(move |_| crate::style::rounded(ground, 8)),
            );
        }
        column![
            row![
                column![
                    form::title(theme, "AutoMod"),
                    form::note(
                        theme,
                        "Set rules that Discord enforces for you, before a message is posted.",
                    ),
                ]
                .spacing(form::TIGHT)
                .width(Length::Fill),
                action(
                    theme,
                    "Add Rule",
                    Kind::Primary,
                    (!saving).then(|| message(Event::AutoMod(AutoModInput::Add))),
                ),
            ]
            .spacing(form::GAP)
            .align_y(Alignment::Center),
            form::rule(theme),
            list,
        ]
        .spacing(form::GAP)
        .width(Length::Fill)
        .into()
    }

    /// The dialog that makes a keyword rule: its name, its words, and
    /// whether it blocks, alerts a channel or times a member out.
    pub(super) fn automod_dialog(
        self,
        page: &'a AutoModPage,
        screen: &'a Screen,
    ) -> Option<Element<'a, Message>> {
        let draft = page.draft.as_ref()?;
        let theme = self.cx.look.theme;
        let core = self.cx.state.server_settings.as_ref()?;
        let channels = self.cx.app.settings_channels(&core.server, false);
        let names: Vec<String> = channels
            .iter()
            .map(|(_, name)| format!("#{name}"))
            .collect();
        let ids: Vec<String> = channels.into_iter().map(|(id, _)| id).collect();
        let chosen = draft
            .form
            .alert_channel
            .as_ref()
            .and_then(|channel| ids.iter().position(|id| id == channel));
        let ready = draft.ready();
        let form = &draft.form;
        let mut body = column![
            text("Add Rule")
                .size(20)
                .font(crate::fonts::BOLD())
                .color(theme.text),
            form::group([
                form::heading(theme, "NAME"),
                form::entry(theme, NAME, "Rule name", form.name.as_str())
                    .on_input(|text| message(Event::AutoMod(AutoModInput::Name(text))))
                    .into(),
            ]),
            form::group([
                form::heading(theme, "KEYWORDS"),
                form::entry(
                    theme,
                    "automod-keywords",
                    "Words or phrases, separated by commas",
                    form.keywords.as_str(),
                )
                .on_input(|text| message(Event::AutoMod(AutoModInput::Keywords(text))))
                .into(),
                form::note(
                    theme,
                    "Use * before or after a word to match it as a prefix or suffix.",
                ),
            ]),
            form::heading(theme, "ACTIONS"),
            form::switch(
                "Block the message",
                form.block,
                Some(message(Event::AutoMod(AutoModInput::Block(!form.block),)))
            )
            .description("Stop the message from being posted.")
            .view(self.cx.look),
            form::switch(
                "Send an alert",
                form.alert,
                Some(message(Event::AutoMod(AutoModInput::Alert(!form.alert),)))
            )
            .description("Post what was caught in a channel only moderators see.")
            .view(self.cx.look),
        ]
        .spacing(14)
        .padding(16);
        if form.alert {
            body = body.push(
                form::dropdown(
                    names,
                    chosen,
                    screen.open == Some(Drop::AutoModChannel),
                    message(Event::Drop(Drop::AutoModChannel)),
                    move |index| {
                        message(Event::AutoMod(AutoModInput::AlertChannel(Some(
                            ids[index].clone(),
                        ))))
                    },
                )
                .view(self.cx.look),
            );
        }
        body = body.push(
            form::switch(
                "Time out the member",
                form.timeout,
                Some(message(Event::AutoMod(AutoModInput::Timeout(
                    !form.timeout,
                )))),
            )
            .description("Time them out for an hour.")
            .view(self.cx.look),
        );
        if let Some(error) = &draft.error {
            body = body.push(form::error(theme, error.as_str()));
        }
        body = body.push(dialog::buttons([
            action(
                theme,
                "Cancel",
                Kind::Ghost,
                (!draft.saving).then(|| message(Event::AutoMod(AutoModInput::Cancel))),
            )
            .into(),
            action(
                theme,
                if draft.saving { "Saving..." } else { "Save" },
                Kind::Primary,
                ready.then(|| message(Event::AutoMod(AutoModInput::Save))),
            )
            .into(),
        ]));
        Some(dialog::panel(
            theme,
            body.into(),
            DIALOG,
            None,
            message(Event::AutoMod(AutoModInput::Cancel)),
        ))
    }
}

pub(super) fn dialog_field(page: &AutoModPage) -> Option<&'static str> {
    page.draft.as_ref().map(|_| NAME)
}
