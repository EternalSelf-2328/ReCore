//! AutoMod, as Discord's server settings list it: the server's rules, each
//! switched on or off and deleted at once, and a new keyword rule with the
//! actions it takes. It takes Manage Server.

use super::*;
use crate::chat::{AutoModAction, AutoModDraft, AutoModRule, ServerSettingsAction};

/// Something done on AutoMod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoModInput {
    SetEnabled {
        id: String,
        enabled: bool,
    },
    /// Add Rule: the dialog.
    Add,
    Name(String),
    Keywords(String),
    Block(bool),
    Alert(bool),
    /// The channel an alert goes to, by Discord id.
    AlertChannel(Option<String>),
    Timeout(bool),
    Save,
    Cancel,
    Delete(String),
}

/// The new rule's form, as its dialog has it.
#[derive(Clone, Debug, Default)]
pub struct RuleDraft {
    pub form: AutoModDraft,
    pub saving: bool,
    pub error: Option<String>,
}

impl RuleDraft {
    /// The words the rule looks for, split at commas and new lines.
    pub fn words(&self) -> Vec<String> {
        self.form
            .keywords
            .split([',', '\n'])
            .map(str::trim)
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// Whether it may be sent, as Discord's rule asks: a name, a word, and
    /// an action.
    pub fn ready(&self) -> bool {
        !self.saving
            && !self.form.name.trim().is_empty()
            && !self.words().is_empty()
            && (self.form.block
                || self.form.timeout
                || (self.form.alert && self.form.alert_channel.is_some()))
    }
}

/// AutoMod, as Discord sent it.
#[derive(Clone, Debug)]
pub struct AutoModPage {
    pub rules: Vec<AutoModRule>,
    pub draft: Option<RuleDraft>,
}

impl AutoModPage {
    pub fn new(rules: Vec<AutoModRule>) -> Self {
        Self { rules, draft: None }
    }
}

impl Application {
    pub(in crate::application) fn automod_input(&mut self, input: AutoModInput) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        if !self.may(&server, Permission::ManageServer) {
            return vec![];
        }
        let Some(page) = self.automod_page() else {
            return vec![];
        };
        let saving = page.draft.as_ref().is_some_and(|draft| draft.saving);
        let action = match input {
            AutoModInput::Add => {
                if !saving {
                    page.draft = Some(RuleDraft {
                        form: AutoModDraft {
                            block: true,
                            ..AutoModDraft::default()
                        },
                        ..RuleDraft::default()
                    });
                }
                return vec![];
            }
            AutoModInput::Name(name) => {
                page.write(|form| &mut form.name, name, 100);
                return vec![];
            }
            AutoModInput::Keywords(words) => {
                page.write(|form| &mut form.keywords, words, 2000);
                return vec![];
            }
            AutoModInput::Block(on) => {
                page.write_draft(|draft| draft.form.block = on);
                return vec![];
            }
            AutoModInput::Alert(on) => {
                page.write_draft(|draft| {
                    draft.form.alert = on;
                    if !on {
                        draft.form.alert_channel = None;
                    }
                });
                return vec![];
            }
            AutoModInput::AlertChannel(channel) => {
                if let Some(draft) = page.draft.as_mut().filter(|draft| !draft.saving) {
                    draft.form.alert_channel = channel;
                    draft.form.alert = true;
                }
                return vec![];
            }
            AutoModInput::Timeout(on) => {
                page.write_draft(|draft| draft.form.timeout = on);
                return vec![];
            }
            AutoModInput::Cancel => {
                if !saving {
                    page.draft = None;
                }
                return vec![];
            }
            AutoModInput::Save => {
                let Some(draft) = page.draft.as_mut().filter(|draft| draft.ready()) else {
                    return vec![];
                };
                draft.saving = true;
                draft.error = None;
                let mut form = draft.form.clone();
                form.name = form.name.trim().to_owned();
                form.keywords = draft.words().join(",");
                AutoModAction::Create(form)
            }
            AutoModInput::SetEnabled { id, enabled } => {
                let Some(rule) = page.rules.iter_mut().find(|rule| rule.id == id) else {
                    return vec![];
                };
                rule.enabled = enabled;
                AutoModAction::SetEnabled { id, enabled }
            }
            AutoModInput::Delete(id) => {
                page.rules.retain(|rule| rule.id != id);
                AutoModAction::Delete(id)
            }
        };
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: ServerSettingsAction::AutoMod(action),
            },
        ))]
    }

    /// What AutoMod asked of Discord, and the server's rules after it.
    pub(in crate::application) fn automod_changed(
        &mut self,
        server: String,
        action: AutoModAction,
        result: Result<(), String>,
        rules: Option<Vec<AutoModRule>>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let mut error = None;
        let Some(page) = self.automod_page() else {
            return;
        };
        if let Some(rules) = rules {
            page.rules = rules;
        }
        match (action, result) {
            (AutoModAction::Create(_) | AutoModAction::Delete(_), result) => match result {
                Ok(()) => page.draft = None,
                Err(why) => {
                    if let Some(draft) = &mut page.draft {
                        draft.saving = false;
                        draft.error = Some(why);
                    } else {
                        error = Some(why);
                    }
                }
            },
            (AutoModAction::SetEnabled { .. }, result) => error = result.err(),
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn automod_page(&mut self) -> Option<&mut AutoModPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::AutoMod(page) => Some(page),
            _ => None,
        }
    }
}

impl AutoModPage {
    fn write(
        &mut self,
        field: impl FnOnce(&mut AutoModDraft) -> &mut String,
        text: String,
        limit: usize,
    ) {
        if let Some(draft) = self.draft.as_mut().filter(|draft| !draft.saving) {
            let field = field(&mut draft.form);
            field.clear();
            field.extend(text.chars().take(limit));
        }
    }

    fn write_draft(&mut self, write: impl FnOnce(&mut RuleDraft)) {
        if let Some(draft) = self.draft.as_mut().filter(|draft| !draft.saving) {
            write(draft);
        }
    }
}
