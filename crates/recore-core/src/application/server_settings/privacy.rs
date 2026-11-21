//! Privacy, as a server's settings show it: whether its members may DM
//! this account, whether their DMs are filtered into message requests, and
//! whether this account's activity shows in the server. Each switch saves
//! at once, and any member may change them.

use super::*;
use crate::chat::{PrivacyChange, PrivacySettings, PrivacyToggle, ServerSettingsAction};

/// Something done on Privacy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivacyInput {
    Set(PrivacyToggle, bool),
}

/// Privacy, as this account's settings say of the server.
#[derive(Clone, Copy, Debug)]
pub struct PrivacyPage {
    pub settings: PrivacySettings,
    pub saving: bool,
}

impl PrivacyPage {
    pub fn new(settings: PrivacySettings) -> Self {
        Self {
            settings,
            saving: false,
        }
    }

    pub fn on(&self, toggle: PrivacyToggle) -> bool {
        match toggle {
            PrivacyToggle::Dms => self.settings.dms,
            PrivacyToggle::MessageRequests => self.settings.message_requests,
            PrivacyToggle::Activity => self.settings.activity,
        }
    }
}

impl Application {
    pub(in crate::application) fn privacy_input(&mut self, input: PrivacyInput) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        let Some(page) = self.privacy_page() else {
            return vec![];
        };
        if page.saving {
            return vec![];
        }
        let PrivacyInput::Set(toggle, on) = input;
        if page.on(toggle) == on {
            return vec![];
        }
        match toggle {
            PrivacyToggle::Dms => page.settings.dms = on,
            PrivacyToggle::MessageRequests => page.settings.message_requests = on,
            PrivacyToggle::Activity => page.settings.activity = on,
        }
        page.saving = true;
        let change = match toggle {
            PrivacyToggle::Dms => PrivacyChange::Dms(on),
            PrivacyToggle::MessageRequests => PrivacyChange::MessageRequests(on),
            PrivacyToggle::Activity => PrivacyChange::Activity(on),
        };
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: ServerSettingsAction::Privacy(change),
            },
        ))]
    }

    /// What Privacy asked of Discord, and the settings after it.
    pub(in crate::application) fn privacy_saved(
        &mut self,
        server: String,
        change: PrivacyChange,
        result: Result<PrivacySettings, String>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let error = result.as_ref().err().cloned();
        let Some(page) = self.privacy_page() else {
            return;
        };
        page.saving = false;
        if let Ok(settings) = result {
            page.settings = settings;
        } else {
            // Put back what Discord has, as Discord's client does.
            match change {
                PrivacyChange::Dms(on) => page.settings.dms = !on,
                PrivacyChange::MessageRequests(on) => page.settings.message_requests = !on,
                PrivacyChange::Activity(on) => page.settings.activity = !on,
            }
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn privacy_page(&mut self) -> Option<&mut PrivacyPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::Privacy(page) => Some(page),
            _ => None,
        }
    }
}
