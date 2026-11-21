//! Discovery: whether the server is listed in Discord's Discovery, what it
//! still needs to be, and its keywords and description. Only its owner
//! moves the switch.

use iced::{
    Element, Length,
    widget::{column, row, text},
};
use recore_core::application::{DiscoveryInput, DiscoveryPage, ServerSettings};

use super::{Event, Screen, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    form,
};

impl<'a> View<'a> {
    pub(super) fn discovery_page(
        self,
        core: &'a ServerSettings,
        page: &'a DiscoveryPage,
        _screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let owner = self.cx.app.owns_server(&core.server);
        let may = page.may_change(owner);
        let settings = &page.settings;
        let mut parts = column![
            form::title(theme, "Discovery"),
            form::note(
                theme,
                "List your server in Discord's Discovery so people looking for a community like yours can find it.",
            ),
            form::switch(
                "List this server in Discovery",
                settings.enabled,
                (may && !page.saving).then(|| {
                    message(Event::Discovery(DiscoveryInput::Set(!settings.enabled)))
                }),
            )
            .description(if settings.disabled {
                "Discord disabled Discovery for this server."
            } else if !settings.sufficient && !settings.enabled {
                "This server doesn't meet Discovery's requirements yet."
            } else if !owner {
                "Only the server's owner can change this."
            } else {
                "Anyone can find this server in Discovery."
            })
            .view(self.cx.look),
        ]
        .spacing(form::GAP);
        if settings.health_score_pending {
            parts = parts.push(form::note(
                theme,
                "Discord is still working out this server's activity.",
            ));
        }
        parts = parts.push(form::heading(theme, "REQUIREMENTS"));
        for (label, met) in [
            ("A safe environment", settings.safe_environment),
            ("An active community", settings.healthy),
            ("Member retention", settings.retention_healthy),
            ("Weekly visitors and talkers", settings.engagement_healthy),
            ("Enough members", settings.size),
            ("Old enough", settings.age),
            (
                "Two-factor authentication for moderators",
                settings.protected,
            ),
            ("A rules channel", settings.valid_rules_channel),
        ] {
            let size = 14;
            parts = parts.push(
                row![
                    text(if met { "✓" } else { "✗" }).size(size).color(if met {
                        theme.live
                    } else {
                        theme.danger
                    }),
                    text(label).size(size).color(theme.secondary),
                ]
                .spacing(8),
            );
        }
        if let Some(size) = settings.minimum_size {
            parts = parts.push(form::note(
                theme,
                format!("Discovery asks for {size} members."),
            ));
        }
        if let Some(days) = settings.minimum_age {
            parts = parts.push(form::note(
                theme,
                format!("Discovery asks for a server {days} days old or older."),
            ));
        }
        let mut details = column![].spacing(form::TIGHT);
        if !settings.keywords.is_empty() {
            details = details.push(form::heading(theme, "KEYWORDS"));
            details = details.push(
                text(settings.keywords.join(", "))
                    .size(14)
                    .color(theme.text),
            );
        }
        if !settings.about.is_empty() {
            details = details.push(form::heading(theme, "ABOUT"));
            details = details.push(
                text(settings.about.as_str())
                    .size(14)
                    .color(theme.secondary),
            );
        }
        details = details.push(form::heading(theme, "LANDING PAGE"));
        details = details.push(
            text(if settings.published {
                "Published"
            } else {
                "Not published"
            })
            .size(14)
            .color(theme.text),
        );
        parts = parts
            .push(form::rule(theme))
            .push(details)
            .spacing(form::GAP)
            .width(Length::Fill);
        if !settings.sufficient_without_grace && settings.enabled {
            parts = parts.push(form::note(
                theme,
                "This server no longer meets the requirements, but its grace period keeps it listed for now.",
            ));
        }
        if !owner {
            parts = parts.push(action(
                theme,
                "Only the owner may change this",
                Kind::Ghost,
                None,
            ));
        }
        parts.into()
    }
}
