//! Linking a Discord account by QR code, or by a token the person pastes,
//! beside the chat backend, so a build without Matrix can link before
//! anything else runs. The login goes into the OS credential store; see
//! `recore_discord::remote_auth`.

use recore_discord::{
    api::{Api, Refused},
    remote_auth::{self, Step, Stopped},
    token,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    discord_accounts::SavedAccount,
    runtime::{BackgroundRuntime, BackgroundTask},
};

/// How a login is asked for: by Discord's QR code, with a pasted token, or
/// with the account's email and password.
pub enum Login {
    Qr,
    Token(String),
    Email { email: String, password: String },
}

/// A login under way: its progress, and the authenticator code an email
/// login's two-factor prompt waits for.
pub struct Started {
    pub task: BackgroundTask<()>,
    pub progress: UnboundedReceiver<Progress>,
    pub codes: UnboundedSender<String>,
}

/// How linking goes, for [`crate::application::Application::linking`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The code to show, as the address it carries.
    Code(String),
    /// Someone scanned it, by their username, and confirms on their phone.
    Scanned(String),
    /// Discord wants a captcha solved, on the page at this address.
    Captcha(String),
    /// Discord wants the code from the account's authenticator app, or a
    /// backup code.
    Mfa,
    /// That code wasn't taken, and another can be tried.
    MfaError(String),
    /// The account is linked and its login saved, under this id.
    Linked(SavedAccount),
    /// The code expired before anyone confirmed it.
    Expired,
    Failed(String),
}

/// Starts linking an account on `runtime`: by QR code, with a pasted token,
/// or with an email and password. Its progress arrives on the receiver until
/// it ends, and dropping the task stops it. An email login's two-factor
/// prompt takes the code sent on `codes`.
pub fn start(runtime: &BackgroundRuntime, login: Login) -> Started {
    let (sender, progress) = mpsc::unbounded_channel();
    let (codes, code_receiver) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    let task = match login {
        Login::Qr => runtime.spawn(run(report)),
        Login::Token(token) => runtime.spawn(with_token(token, report)),
        Login::Email { email, password } => {
            runtime.spawn(with_email(email, password, code_receiver, report))
        }
    };
    Started {
        task,
        progress,
        codes,
    }
}

/// Signs in with Discord's email and password, asking for the authenticator
/// code of an account that has two-factor authentication, then saves the
/// login under the account it belongs to.
async fn with_email(
    email: String,
    password: String,
    mut codes: UnboundedReceiver<String>,
    progress: impl Fn(Progress),
) {
    let mut shown = |page: &str| progress(Progress::Captcha(page.to_owned()));
    let signed_in = match recore_discord::auth::log_in(&email, &password, &mut shown).await {
        Ok(signed_in) => signed_in,
        Err(error) => return progress(Progress::Failed(login_failed(&error))),
    };
    let token = match signed_in {
        recore_discord::auth::Login::Token(token) => token,
        recore_discord::auth::Login::Mfa {
            ticket,
            login_instance_id,
            ..
        } => {
            progress(Progress::Mfa);
            let mut tries = 0;
            loop {
                let Some(code) = codes.recv().await else {
                    return;
                };
                let code = code.trim();
                if code.is_empty() {
                    progress(Progress::Mfa);
                    continue;
                }
                match recore_discord::auth::confirm_mfa(
                    &ticket,
                    &login_instance_id,
                    code,
                    &mut shown,
                )
                .await
                {
                    Ok(token) => break token,
                    Err(error) if tries < 2 => {
                        tries += 1;
                        progress(Progress::MfaError(login_failed(&error)));
                    }
                    Err(error) => return progress(Progress::Failed(login_failed(&error))),
                }
            }
        }
    };
    progress(match save(token).await {
        Ok(account) => Progress::Linked(account),
        Err(error) => Progress::Failed(login_failed(&error)),
    });
}

/// Why an email login stopped, in words that never repeat the email or
/// password.
fn login_failed(error: &anyhow::Error) -> String {
    if matches!(
        error
            .downcast_ref::<Refused>()
            .map(|refused| refused.0.as_u16()),
        Some(401 | 403)
    ) {
        return "Discord didn't accept that email and password, or the code.".into();
    }
    tracing::warn!(error = format!("{error:#}"), "could not sign in to Discord");
    format!("ReCore couldn't sign in to Discord: {error}")
}

/// Links the account a pasted token belongs to, once Discord accepts it.
async fn with_token(token: String, progress: impl Fn(Progress)) {
    let token = match token::clean(&token) {
        Ok(token) => token,
        Err(error) => return progress(Progress::Failed(error.to_string())),
    };
    progress(match save(token).await {
        Ok(account) => Progress::Linked(account),
        Err(error) => match error.downcast_ref::<Refused>() {
            Some(Refused(status)) if status.as_u16() == 401 => {
                Progress::Failed("Discord didn't accept that token.".into())
            }
            _ => {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "could not link Discord by token"
                );
                Progress::Failed(format!("ReCore couldn't link Discord: {error}"))
            }
        },
    });
}

/// Saves `token` under the account it belongs to, discovering the account's
/// id and name from Discord, as linking by QR code and by token both do.
async fn save(token: String) -> anyhow::Result<SavedAccount> {
    let api = Api::new(token.clone())?;
    api.learn_build_number().await;
    let user = api.me().await?;
    token::store_account(&user.id, &token)?;
    Ok(SavedAccount {
        id: user.id,
        name: user.global_name.unwrap_or_else(|| user.username.clone()),
        username: user.username,
    })
}

/// Links an account, telling `progress` how it goes until it ends.
async fn run(progress: impl Fn(Progress)) {
    let linked = remote_auth::link(|step| {
        progress(match step {
            Step::Code(address) => Progress::Code(address),
            Step::Scanned { name, .. } => Progress::Scanned(name),
            Step::Captcha(page) => Progress::Captcha(page),
        });
    })
    .await;
    let account = match linked {
        Ok(login) => save(login).await,
        Err(error) => Err(error),
    };
    progress(match account {
        Ok(account) => Progress::Linked(account),
        Err(error) => match error.downcast_ref::<Stopped>() {
            Some(Stopped::Expired) => Progress::Expired,
            Some(stopped) => Progress::Failed(stopped.to_string()),
            None => {
                tracing::warn!(error = format!("{error:#}"), "could not link Discord");
                Progress::Failed(format!("ReCore couldn't link Discord: {error}"))
            }
        },
    });
}
