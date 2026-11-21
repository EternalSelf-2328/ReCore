//! Integrations, as Discord's server settings list them: the bots and
//! services integrated with the server, each removed at once. It takes
//! Manage Server.

use super::*;
use crate::chat::{IntegrationAction, ServerIntegration};

/// Something done on Integrations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationInput {
    /// The X on an integration.
    Delete(String),
}

/// Integrations, as Discord sent them.
#[derive(Clone, Debug)]
pub struct IntegrationsPage {
    pub integrations: Vec<ServerIntegration>,
}

impl IntegrationsPage {
    pub fn new(integrations: Vec<ServerIntegration>) -> Self {
        Self { integrations }
    }
}

impl Application {
    pub(in crate::application) fn integration_input(
        &mut self,
        input: IntegrationInput,
    ) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        if !self.may(&server, Permission::ManageServer) {
            return vec![];
        }
        let Some(page) = self.integration_page() else {
            return vec![];
        };
        let IntegrationInput::Delete(id) = input;
        // Gone at once, as in Discord's client.
        page.integrations.retain(|integration| integration.id != id);
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: ServerSettingsAction::Integrations(IntegrationAction::Delete(id)),
            },
        ))]
    }

    /// What Integrations asked of Discord, and the server's integrations
    /// after it.
    pub(in crate::application) fn integrations_changed(
        &mut self,
        server: String,
        result: Result<(), String>,
        integrations: Option<Vec<ServerIntegration>>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let error = result.err();
        let Some(page) = self.integration_page() else {
            return;
        };
        if let Some(integrations) = integrations {
            page.integrations = integrations;
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn integration_page(&mut self) -> Option<&mut IntegrationsPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::Integrations(page) => Some(page),
            _ => None,
        }
    }
}
