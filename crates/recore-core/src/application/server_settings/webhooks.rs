//! Webhooks, as Discord's server settings list them: the server's webhooks,
//! each deleted at once or copied by its URL, and a new one from a name and
//! a channel. It takes Manage Webhooks.

use super::*;
use crate::chat::{ServerWebhook, WebhookAction};

pub const WEBHOOK_NAME_LIMIT: usize = 80;

/// Something done on Webhooks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebhookInput {
    /// Create Webhook: the dialog.
    Create,
    Name(String),
    Channel(String),
    Save,
    Cancel,
    Delete(String),
}

/// A new webhook, as its dialog has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebhookDraft {
    pub name: String,
    /// Its channel's Discord id.
    pub channel: Option<String>,
    pub saving: bool,
    pub error: Option<String>,
}

impl WebhookDraft {
    /// Whether it may be sent: a name of 1 to 80, and a channel.
    pub fn ready(&self) -> bool {
        !self.saving
            && !self.name.trim().is_empty()
            && self.name.trim().chars().count() <= WEBHOOK_NAME_LIMIT
            && self.channel.is_some()
    }
}

/// Webhooks, as Discord sent them.
#[derive(Clone, Debug)]
pub struct WebhooksPage {
    pub webhooks: Vec<ServerWebhook>,
    pub draft: Option<WebhookDraft>,
}

impl WebhooksPage {
    pub fn new(webhooks: Vec<ServerWebhook>) -> Self {
        Self {
            webhooks,
            draft: None,
        }
    }

    /// The webhook's address, which its X copies and its row shows.
    pub fn address(webhook: &ServerWebhook) -> Option<String> {
        webhook.url.clone().or_else(|| {
            let token = webhook.token.as_deref()?;
            Some(format!(
                "https://discord.com/api/webhooks/{}/{}",
                webhook.id, token
            ))
        })
    }
}

impl Application {
    pub(in crate::application) fn webhook_input(&mut self, input: WebhookInput) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        let manage = self.may(&server, Permission::ManageWebhooks);
        let Some(page) = self.webhook_page() else {
            return vec![];
        };
        let saving = page.draft.as_ref().is_some_and(|draft| draft.saving);
        let action = match input {
            WebhookInput::Create => {
                if manage && !saving {
                    page.draft = Some(WebhookDraft::default());
                }
                return vec![];
            }
            WebhookInput::Name(name) => {
                if let Some(draft) = page.draft.as_mut().filter(|draft| !draft.saving) {
                    draft.name = name.chars().take(WEBHOOK_NAME_LIMIT).collect();
                }
                return vec![];
            }
            WebhookInput::Channel(channel) => {
                if let Some(draft) = page.draft.as_mut().filter(|draft| !draft.saving) {
                    draft.channel = Some(channel);
                }
                return vec![];
            }
            WebhookInput::Cancel => {
                if !saving {
                    page.draft = None;
                }
                return vec![];
            }
            WebhookInput::Save => {
                let Some(draft) = page.draft.as_mut().filter(|draft| draft.ready()) else {
                    return vec![];
                };
                draft.saving = true;
                draft.error = None;
                WebhookAction::Create {
                    channel: draft.channel.clone().unwrap_or_default(),
                    name: draft.name.trim().to_owned(),
                }
            }
            WebhookInput::Delete(id) => {
                if !manage {
                    return vec![];
                }
                page.webhooks.retain(|webhook| webhook.id != id);
                WebhookAction::Delete(id)
            }
        };
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: ServerSettingsAction::Webhooks(action),
            },
        ))]
    }

    /// What Webhooks asked of Discord, and the server's webhooks after it.
    pub(in crate::application) fn webhooks_changed(
        &mut self,
        server: String,
        action: WebhookAction,
        result: Result<(), String>,
        webhooks: Option<Vec<ServerWebhook>>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let mut error = None;
        let Some(page) = self.webhook_page() else {
            return;
        };
        if let Some(webhooks) = webhooks {
            page.webhooks = webhooks;
        }
        match (action, result) {
            (WebhookAction::Delete(_), result) => error = result.err(),
            (_, Ok(())) => page.draft = None,
            (_, Err(why)) => {
                if let Some(draft) = &mut page.draft {
                    draft.saving = false;
                    draft.error = Some(why);
                }
            }
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn webhook_page(&mut self) -> Option<&mut WebhooksPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::Webhooks(page) => Some(page),
            _ => None,
        }
    }
}
