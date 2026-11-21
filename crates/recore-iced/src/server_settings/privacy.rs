//! Privacy: whether a server's members may DM this account, whether their
//! DMs are filtered into message requests, and whether this account's
//! activity shows in the server. Each switch saves at once.

use iced::{Element, Length, widget::column};
use recore_core::{
    application::{PrivacyInput, PrivacyPage, ServerSettings},
    chat::PrivacyToggle,
};

use super::{Event, Screen, View, message};
use crate::{app::Message, form};

impl<'a> View<'a> {
    pub(super) fn privacy_page(
        self,
        _core: &'a ServerSettings,
        page: &'a PrivacyPage,
        _screen: &'a Screen,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let ready = !page.saving;
        let switch = |toggle: PrivacyToggle, on: bool| {
            form::switch(
                match toggle {
                    PrivacyToggle::Dms => "Direct Messages",
                    PrivacyToggle::MessageRequests => "Message Requests",
                    PrivacyToggle::Activity => "Activity Status",
                },
                on,
                ready.then(|| message(Event::Privacy(PrivacyInput::Set(toggle, !on)))),
            )
            .description(match toggle {
                PrivacyToggle::Dms => "Allow direct messages from other members of this server.",
                PrivacyToggle::MessageRequests => {
                    "Show direct messages from this server in your message requests."
                }
                PrivacyToggle::Activity => {
                    "Show your current activity to other members of this server."
                }
            })
            .view(self.cx.look)
        };
        column![
            form::title(theme, "Privacy"),
            form::note(
                theme,
                "Choose how other members of this server can reach you and what they see of you.",
            ),
            switch(PrivacyToggle::Dms, page.settings.dms),
            switch(
                PrivacyToggle::MessageRequests,
                page.settings.message_requests
            ),
            switch(PrivacyToggle::Activity, page.settings.activity),
        ]
        .spacing(form::GAP)
        .width(Length::Fill)
        .into()
    }
}
