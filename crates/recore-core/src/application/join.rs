//! Joining a Discord server by an invite: Join a Server, from the "+" at the
//! foot of the server rail, takes a pasted link or code, and a clicked
//! invite link opens the same dialog with it. Either looks the invite up
//! and shows its server before anyone joins; joining waits for the click.

use super::*;
use crate::chat::{InvitePreview, JoinedServer};

/// What a membership screening question was answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationAnswer {
    /// Agree or disagree with the terms at this field.
    Terms(usize),
    /// Type this into the text question at this field.
    Text(usize, String),
    /// Choose this option of the question at this field.
    Choice(usize, u64),
}

/// The Join a Server dialog, while it's open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JoinDialog {
    /// The code of the invite looked up or shown.
    pub code: Option<String>,
    /// What the invite shows, once Discord answered.
    pub preview: Option<InvitePreview>,
    /// This account is in the server already, so the dialog opens it.
    pub member: bool,
    /// A clicked invite link opened the dialog, which then has no field.
    pub from_link: bool,
    pub resolving: bool,
    pub joining: bool,
    /// The page with the captcha Discord asked for before joining.
    pub captcha: Option<String>,
    /// The server, joined, asks new members to fill in its form first.
    pub verification: Option<crate::chat::VerificationForm>,
    pub error: Option<String>,
}

impl Application {
    /// Opens Join a Server, empty; nothing without a linked Discord account.
    pub(super) fn open_join(&mut self) -> Vec<Effect> {
        if self.state.discord_online || self.state.preview {
            self.state.join = Some(JoinDialog::default());
        }
        vec![]
    }

    /// Looks up the invite in what someone typed or pasted into Join a
    /// Server.
    pub(super) fn find_invite(&mut self, typed: String) -> Vec<Effect> {
        let Some(join) = self.state.join.as_mut().filter(|join| !join.joining) else {
            return vec![];
        };
        let Some(code) = typed_code(&typed) else {
            join.error = Some("Paste an invite link, such as https://discord.gg/recore.".into());
            return vec![];
        };
        *join = JoinDialog {
            code: Some(code.clone()),
            resolving: true,
            ..JoinDialog::default()
        };
        self.resolve(code, Some(typed))
    }

    /// Opens the dialog for a clicked invite link, by its code, and looks
    /// the invite up.
    pub(super) fn open_invite_link(&mut self, code: String) -> Vec<Effect> {
        if !self.state.discord_online && !self.state.preview {
            return vec![];
        }
        self.state.join = Some(JoinDialog {
            code: Some(code.clone()),
            from_link: true,
            resolving: true,
            ..JoinDialog::default()
        });
        self.resolve(code, None)
    }

    fn resolve(&self, code: String, typed: Option<String>) -> Vec<Effect> {
        if self.state.preview {
            return vec![];
        }
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ResolveInvite { code, typed },
        ))]
    }

    pub(super) fn invite_resolved(
        &mut self,
        code: String,
        result: Result<InvitePreview, String>,
    ) -> Vec<Effect> {
        let Some(join) = self
            .state
            .join
            .as_mut()
            .filter(|join| join.resolving && join.code.as_ref() == Some(&code))
        else {
            return vec![];
        };
        join.resolving = false;
        let preview = match result {
            Ok(preview) => preview,
            Err(error) => {
                join.error = Some(error);
                return vec![];
            }
        };
        join.member = self
            .state
            .servers
            .iter()
            .any(|server| server.id == preview.server);
        // A clicked link to a server this account is in goes there, as in
        // Discord.
        if join.member && join.from_link {
            return self.open_joined(&preview.server, preview.room);
        }
        join.preview = Some(preview);
        vec![]
    }

    /// Joins by the invite the dialog shows, or opens its server for a
    /// member.
    pub(super) fn join_server(&mut self) -> Vec<Effect> {
        let Some(join) = self
            .state
            .join
            .as_mut()
            .filter(|join| !join.joining && join.verification.is_none())
        else {
            return vec![];
        };
        let Some(preview) = join.preview.clone() else {
            return vec![];
        };
        if join.member {
            return self.open_joined(&preview.server, preview.room);
        }
        if self.state.preview {
            return vec![];
        }
        join.joining = true;
        join.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::JoinServer {
                preview: Box::new(preview),
                from_link: join.from_link,
            },
        ))]
    }

    pub(super) fn join_captcha(&mut self, code: String, page: String) -> Vec<Effect> {
        let Some(join) = self
            .state
            .join
            .as_mut()
            .filter(|join| join.joining && join.code.as_ref() == Some(&code))
        else {
            return vec![];
        };
        join.captcha = Some(page.clone());
        vec![Effect::OpenPage(page)]
    }

    /// Opens the captcha Discord asked for again, as linking's does.
    pub(super) fn open_join_captcha(&self) -> Vec<Effect> {
        match self
            .state
            .join
            .as_ref()
            .and_then(|join| join.captcha.clone())
        {
            Some(page) => vec![Effect::OpenPage(page)],
            None => vec![],
        }
    }

    pub(super) fn server_joined(
        &mut self,
        code: String,
        result: Result<JoinedServer, String>,
    ) -> Vec<Effect> {
        let Some(join) = self
            .state
            .join
            .as_mut()
            .filter(|join| join.joining && join.code.as_ref() == Some(&code))
        else {
            return vec![];
        };
        join.joining = false;
        join.captcha = None;
        let joined = match result {
            Ok(joined) => joined,
            Err(error) => {
                join.error = Some(error);
                return vec![];
            }
        };
        if joined.form {
            // The server lists once Discord sends it, and the dialog stays
            // to ask for its membership screening form.
            let room = joined.room;
            join.verification = Some(crate::chat::VerificationForm {
                room: room.clone(),
                loading: true,
                ..Default::default()
            });
            self.pending_open = Some(room.clone());
            return vec![Effect::Backend(Command::Discord(
                DiscordCommand::MemberVerification { room },
            ))];
        }
        let mut effects = self.open_joined(&joined.server, joined.room.clone());
        effects.extend(self.fetch_onboarding(joined.room));
        effects
    }

    /// Asks the server joining opened for its onboarding questions, which
    /// show over the server once they come.
    pub(super) fn fetch_onboarding(&mut self, room: String) -> Vec<Effect> {
        if self.state.preview {
            return vec![];
        }
        self.state.onboarding = Some(crate::chat::OnboardingForm {
            room: room.clone(),
            loading: true,
            ..Default::default()
        });
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::GuildOnboarding { room },
        ))]
    }

    /// The onboarding questions arrived, or why they didn't. A server that
    /// asks nothing, or whose answers this account already gave, is left
    /// alone.
    pub(super) fn guild_onboarding(
        &mut self,
        room: String,
        result: Result<crate::chat::OnboardingForm, String>,
    ) -> Vec<Effect> {
        let Some(form) = self
            .state
            .onboarding
            .as_mut()
            .filter(|form| form.loading && form.room == room)
        else {
            return vec![];
        };
        let found = match result {
            Ok(found) => found,
            Err(_) => {
                self.state.onboarding = None;
                return vec![];
            }
        };
        if !found.asked() || found.answered() {
            self.state.onboarding = None;
            return vec![];
        }
        *form = found;
        vec![]
    }

    /// Chooses or unchoses an onboarding option.
    pub(super) fn answer_onboarding(&mut self, prompt: usize, option: usize) -> Vec<Effect> {
        let Some(form) = self.state.onboarding.as_mut() else {
            return vec![];
        };
        let Some(prompt) = form.prompts.get_mut(prompt) else {
            return vec![];
        };
        let Some(choice) = prompt.options.get(option).map(|option| option.id.clone()) else {
            return vec![];
        };
        if prompt.single_select {
            prompt.selected.clear();
            prompt.selected.push(choice);
        } else if let Some(chosen) = prompt
            .selected
            .iter()
            .position(|selected| selected == &choice)
        {
            prompt.selected.remove(chosen);
        } else {
            prompt.selected.push(choice);
        }
        form.error = None;
        vec![]
    }

    /// Sends the onboarding answers, or skips a server that requires none.
    pub(super) fn submit_onboarding(&mut self, skip: bool) -> Vec<Effect> {
        let Some(form) = self.state.onboarding.as_mut() else {
            return vec![];
        };
        if form.loading || form.submitting || (!skip && !form.ready()) {
            return vec![];
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or_default();
        let prompts_seen: serde_json::Map<String, serde_json::Value> = form
            .prompts
            .iter()
            .map(|prompt| (prompt.id.clone(), serde_json::Value::from(now)))
            .collect();
        let responses_seen: serde_json::Map<String, serde_json::Value> = form
            .prompts
            .iter()
            .flat_map(|prompt| prompt.selected.iter())
            .map(|option| (option.clone(), serde_json::Value::from(now)))
            .collect();
        let responses = if skip { Vec::new() } else { form.responses() };
        let room = form.room.clone();
        form.submitting = true;
        form.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SubmitOnboarding {
                room,
                body: serde_json::json!({
                    "onboarding_responses": responses,
                    "onboarding_prompts_seen": prompts_seen,
                    "onboarding_responses_seen": responses_seen,
                }),
            },
        ))]
    }

    /// Whether onboarding answers were sent, or why not.
    pub(super) fn onboarding_submitted(
        &mut self,
        room: String,
        result: Result<(), String>,
    ) -> Vec<Effect> {
        let Some(form) = self
            .state
            .onboarding
            .as_mut()
            .filter(|form| form.room == room)
        else {
            return vec![];
        };
        form.submitting = false;
        match result {
            Ok(()) => self.state.onboarding = None,
            Err(why) => form.error = Some(why),
        }
        vec![]
    }

    /// The membership screening form arrived, or why it didn't.
    pub(super) fn member_verification(
        &mut self,
        room: String,
        result: Result<crate::chat::VerificationForm, String>,
    ) -> Vec<Effect> {
        let Some(join) = &mut self.state.join else {
            return vec![];
        };
        let Some(form) = &mut join.verification else {
            return vec![];
        };
        if form.room != room {
            return vec![];
        }
        match result {
            Ok(found) => *form = found,
            Err(error) => {
                form.loading = false;
                form.error = Some(error);
            }
        }
        vec![]
    }

    pub(super) fn answer_verification(&mut self, answer: VerificationAnswer) -> Vec<Effect> {
        let Some(form) = self
            .state
            .join
            .as_mut()
            .and_then(|join| join.verification.as_mut())
        else {
            return vec![];
        };
        match answer {
            VerificationAnswer::Terms(index) => {
                if let Some(field) = form.fields.get_mut(index)
                    && field.is_terms()
                {
                    field.agreed = !field.agreed;
                }
            }
            VerificationAnswer::Text(index, text) => {
                if let Some(field) = form.fields.get_mut(index)
                    && field.is_text()
                {
                    field.text = text;
                }
            }
            VerificationAnswer::Choice(index, choice) => {
                if let Some(field) = form.fields.get_mut(index)
                    && field.is_choice()
                {
                    field.choice = Some(choice);
                }
            }
        }
        form.error = None;
        vec![]
    }

    pub(super) fn submit_verification(&mut self) -> Vec<Effect> {
        let Some(form) = self
            .state
            .join
            .as_mut()
            .and_then(|join| join.verification.as_mut())
        else {
            return vec![];
        };
        if !form.ready() {
            return vec![];
        }
        let fields: Vec<serde_json::Value> = form
            .fields
            .iter()
            .map(|field| {
                serde_json::json!({
                    "field_type": field.kind.as_str(),
                    "label": field.label.as_str(),
                    "required": field.required,
                    "values": &field.values,
                    "response": field.response(),
                })
            })
            .collect();
        let room = form.room.clone();
        form.submitting = true;
        form.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SubmitJoinRequest {
                room,
                fields: serde_json::Value::Array(fields),
            },
        ))]
    }

    /// Closes the dialog and opens the invite's channel, now or once it
    /// lists, or else its server.
    pub(super) fn open_joined(&mut self, server: &str, room: String) -> Vec<Effect> {
        self.state.join = None;
        if self.state.rooms.iter().any(|listed| listed.id == room) {
            return self.dispatch(Action::Open(room));
        }
        if self.state.servers.iter().any(|listed| listed.id == server) {
            return self.dispatch(Action::Navigate(Navigation::Server(server.to_owned())));
        }
        self.pending_open = Some(room);
        vec![]
    }
}

/// The invite code in what someone typed, as Discord reads it.
fn typed_code(typed: &str) -> Option<String> {
    #[cfg(feature = "discord")]
    {
        recore_discord::invite::typed_code(typed).map(str::to_owned)
    }
    #[cfg(not(feature = "discord"))]
    {
        let _ = typed;
        None
    }
}
