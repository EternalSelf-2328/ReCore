//! Discovery, as Discord's server settings show it: whether the server is
//! listed, what it still needs to be, and its keywords and description.
//! Only the owner may enable it, as Discord's client decides.

use super::*;
use crate::chat::{DiscoveryAction, DiscoverySettings, ServerSettingsAction};

/// Something done on Discovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryInput {
    Set(bool),
}

/// Discovery, as Discord answered.
#[derive(Clone, Debug)]
pub struct DiscoveryPage {
    pub settings: DiscoverySettings,
    pub saving: bool,
}

impl DiscoveryPage {
    pub fn new(settings: DiscoverySettings) -> Self {
        Self {
            settings,
            saving: false,
        }
    }

    /// Whether this account may move the switch: the server's owner, while
    /// Discovery isn't permanently disabled, and the server meets what
    /// Discovery asks or is already in it.
    pub fn may_change(&self, owner: bool) -> bool {
        owner
            && !self.settings.disabled
            && (self.settings.enabled || self.settings.sufficient)
            && !self.saving
    }
}

impl Application {
    pub(in crate::application) fn discovery_input(&mut self, input: DiscoveryInput) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        let owner = self.owns_server(&server);
        let Some(page) = self.discovery_page() else {
            return vec![];
        };
        let DiscoveryInput::Set(on) = input;
        if !page.may_change(owner) || page.settings.enabled == on {
            return vec![];
        }
        page.settings.enabled = on;
        page.saving = true;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: ServerSettingsAction::Discovery(DiscoveryAction::SetEnabled(on)),
            },
        ))]
    }

    /// What Discovery asked of Discord, and its page after it.
    pub(in crate::application) fn discovery_changed(
        &mut self,
        server: String,
        result: Result<DiscoverySettings, String>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let error = result.as_ref().err().cloned();
        let Some(page) = self.discovery_page() else {
            return;
        };
        page.saving = false;
        match result {
            Ok(settings) => page.settings = settings,
            Err(_) => page.settings.enabled = !page.settings.enabled,
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn discovery_page(&mut self) -> Option<&mut DiscoveryPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::Discovery(page) => Some(page),
            _ => None,
        }
    }

    /// Whether this account owns a server.
    pub fn owns_server(&self, server: &str) -> bool {
        self.server_owner(server).as_deref() == self.state.discord_user.as_deref()
    }
}
