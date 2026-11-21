//! Integrations: the bots and services integrated with the server, each
//! removed at once as Discord's X does. It takes Manage Server.

use iced::{
    Alignment, Element, Length,
    widget::{column, row, text},
};
use recore_core::application::{IntegrationInput, IntegrationsPage, ServerSettings};

use super::{Event, Screen, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    form,
};

impl<'a> View<'a> {
    /// Integrations: each integration with what it is and its X.
    pub(super) fn integrations_page(
        self,
        _core: &'a ServerSettings,
        page: &'a IntegrationsPage,
        _screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut list = column![].spacing(6).width(Length::Fill);
        if page.integrations.is_empty() {
            list = list.push(form::note(theme, "This server has no integrations."));
        }
        for integration in &page.integrations {
            let kind = match integration.kind.as_str() {
                "discord" => "Discord app",
                "twitch" => "Twitch",
                "youtube" => "YouTube",
                "guild_subscription" => "Role subscription",
                other => other,
            };
            let mut under = vec![kind.to_owned()];
            if !integration.account.is_empty() && integration.account != integration.name {
                under.push(integration.account.clone());
            }
            if let Some(count) = integration.subscriber_count {
                under.push(format!("{count} subscribers"));
            }
            if !integration.enabled {
                under.push("Disabled".into());
            }
            let ground = theme.wash(0.03);
            list = list.push(
                iced::widget::container(
                    row![
                        column![
                            text(integration.name.as_str()).size(15).color(theme.text),
                            text(under.join(" · ")).size(13).color(theme.muted),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        action(
                            theme,
                            "Delete",
                            Kind::Danger,
                            Some(message(Event::Integrations(IntegrationInput::Delete(
                                integration.id.clone()
                            )))),
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
            form::title(theme, "Integrations"),
            form::note(
                theme,
                "Integrations let apps and services work in this server. Removing one takes away its commands and webhooks.",
            ),
            form::rule(theme),
            list,
        ]
        .spacing(form::GAP)
        .width(Length::Fill)
        .into()
    }
}
