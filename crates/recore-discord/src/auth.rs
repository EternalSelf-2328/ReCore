//! Signing in with Discord's email and password, as its own client's login
//! page does. The password is used for the request and dropped; only the
//! login token is kept, in the OS credential store like one from a QR code.
//! Discord may ask for a captcha, which [`crate::captcha`] shows in the
//! browser, and for a two-factor code, which the caller asks the person for.

use anyhow::Result;

use crate::{api::Api, captcha};

pub use crate::api::Login;

/// Signs in with `email` and `password`, showing each captcha Discord asks
/// for at the page `shown` hears. The answer is either the login or the
/// two-factor prompt to answer with [`confirm_mfa`].
pub async fn log_in(email: &str, password: &str, mut shown: impl FnMut(&str)) -> Result<Login> {
    let api = Api::anonymous()?;
    api.learn_build_number().await;
    captcha::solving(|solved| api.login(email, password, solved), &mut shown).await
}

/// Answers a two-factor prompt with a code from an authenticator app, or a
/// backup code, and gets the login. A captcha Discord asks for on the way is
/// shown as [`log_in`] shows one.
pub async fn confirm_mfa(
    ticket: &str,
    instance: &str,
    code: &str,
    mut shown: impl FnMut(&str),
) -> Result<String> {
    let api = Api::anonymous()?;
    captcha::solving(
        |solved| api.login_mfa(ticket, instance, code, solved),
        &mut shown,
    )
    .await
}
