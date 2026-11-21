//! The Discord tokens live in the OS credential store, next to ReCore's Matrix
//! session, and never in Matrix or in a file. Each linked account's token is
//! kept under its Discord user id, so several accounts can stay linked and
//! one is chosen without scanning a QR code again. So does the key
//! [`crate::cache`] encrypts its files with.

use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock, PoisonError},
};

use anyhow::{Result, anyhow, bail};
use keyring::Entry;

const SERVICE: &str = "io.github.leriart.ReCore";
/// The linked Discord account, set by linking by QR code or by the developer
/// probe's store-token command. Accounts linked since are kept under their
/// own names; this one is read as the account linked before them, and moved
/// under its id when its id is learned.
const ACCOUNT: &str = "discord-token";
/// Where the first probe kept the token; moved to `ACCOUNT` when read.
const EARLIER_ACCOUNT: &str = "discord-probe-token";
/// The key of the files [`crate::cache`] keeps, in hex.
const CACHE_KEY: &str = "discord-cache-key";

/// The account whose token [`store`] replaces when Discord refreshes a login,
/// set by the service as it starts: the one in use this run.
fn current() -> &'static Mutex<Option<String>> {
    static CURRENT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    CURRENT.get_or_init(|| Mutex::new(None))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Makes `id` the account the service is using, so a refreshed token goes to
/// its key. `None` means the account linked before accounts were kept apart.
pub fn set_current(id: Option<&str>) {
    *lock(current()) = id.map(str::to_owned);
}

/// The account the service is using, if it was started for one.
pub fn current_account() -> Option<String> {
    lock(current()).clone()
}

fn entry() -> Result<Entry> {
    named(ACCOUNT)
}

fn named(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(|_| anyhow!("could not access the OS credential store"))
}

/// The credential store's name for an account's token.
fn account_name(id: &str) -> String {
    format!("discord-token-{id}")
}

/// With the test server, the links last while ReCore runs. It starts linked,
/// with [`crate::client::TEST_TOKEN`], unless `RECORE_DISCORD_TEST_UNLINKED` is
/// set, which shows linking against the fake.
fn test_link() -> &'static Mutex<Option<String>> {
    static LINK: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LINK.get_or_init(|| {
        let unlinked = std::env::var_os("RECORE_DISCORD_TEST_UNLINKED").is_some();
        Mutex::new((!unlinked).then(|| crate::client::TEST_TOKEN.to_owned()))
    })
}

/// Links by account with the test server, by user id.
fn test_accounts() -> &'static Mutex<BTreeMap<String, String>> {
    static ACCOUNTS: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();
    ACCOUNTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// No Discord account is linked on this device, which [`load`] says rather
/// than failing.
#[derive(Debug)]
pub struct NotLinked;

impl std::fmt::Display for NotLinked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no Discord account is linked")
    }
}

impl std::error::Error for NotLinked {}

/// An account the window lists, whose token the credential store no longer
/// has, as when it was removed on another run.
fn missing() -> anyhow::Error {
    anyhow!("that Discord account isn't saved on this device anymore")
}

/// Keeps `token` for the account in use, which [`set_current`] named, or for
/// the account linked before accounts were kept apart.
pub fn store(token: &str) -> Result<()> {
    let token = clean(token)?;
    if let Some(id) = current_account() {
        return store_account(&id, &token);
    }
    if crate::client::test_server().is_some() {
        *lock(test_link()) = Some(token);
        return Ok(());
    }
    entry()?
        .set_password(&token)
        .map_err(|_| anyhow!("could not save the token in the OS credential store"))
}

/// Keeps `token` for the Discord account `id`, so switching to it needs no
/// scan or password again.
pub fn store_account(id: &str, token: &str) -> Result<()> {
    let token = clean(token)?;
    if crate::client::test_server().is_some() {
        lock(test_accounts()).insert(id.to_owned(), token);
        return Ok(());
    }
    named(&account_name(id))?
        .set_password(&token)
        .map_err(|_| anyhow!("could not save the token in the OS credential store"))
}

/// The token of the account in use, which [`set_current`] named, or the
/// account linked before accounts were kept apart.
pub fn load() -> Result<String> {
    if let Some(id) = current_account() {
        return load_account(&id);
    }
    if crate::client::test_server().is_some() {
        return lock(test_link()).clone().ok_or_else(|| NotLinked.into());
    }
    match entry()?.get_password() {
        Ok(token) => return Ok(token),
        Err(keyring::Error::NoEntry) => {}
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
    match named(EARLIER_ACCOUNT)?.get_password() {
        Ok(token) => {
            store(&token)?;
            let _ = named(EARLIER_ACCOUNT)?.delete_credential();
            Ok(token)
        }
        Err(keyring::Error::NoEntry) => Err(NotLinked.into()),
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
}

/// The token of the Discord account `id`, kept when it was linked.
pub fn load_account(id: &str) -> Result<String> {
    if crate::client::test_server().is_some() {
        return lock(test_accounts()).get(id).cloned().ok_or_else(missing);
    }
    match named(&account_name(id))?.get_password() {
        Ok(token) => Ok(token),
        Err(keyring::Error::NoEntry) => Err(missing()),
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
}

/// Moves the account linked before accounts were kept apart under its id,
/// once a session says which id it is, so the next start can choose it.
pub fn adopt_legacy(id: &str) -> Result<()> {
    if crate::client::test_server().is_some() {
        let legacy = lock(test_link()).take();
        if let Some(token) = legacy {
            lock(test_accounts()).insert(id.to_owned(), token);
        }
        set_current(Some(id));
        return Ok(());
    }
    let entry = entry()?;
    match entry.get_password() {
        Ok(token) => {
            store_account(id, &token)?;
            let _ = entry.delete_credential();
            set_current(Some(id));
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Err(NotLinked.into()),
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
}

/// The key [`crate::cache`] encrypts its files with, made the first time.
/// With the test server, always the same one, so what a run kept shows in
/// the next, as with Discord.
pub(crate) fn cache_key() -> Result<crate::cache::Key> {
    if crate::client::test_server().is_some() {
        return Ok([7; 32]);
    }
    let entry = named(CACHE_KEY)?;
    match entry.get_password() {
        Ok(hex) => {
            if let Some(key) = crate::cache::read_key(&hex) {
                return Ok(key);
            }
        }
        Err(keyring::Error::NoEntry) => {}
        Err(_) => bail!("could not read the cache key from the OS credential store"),
    }
    // None yet, or one that isn't a key: what it encrypted can't be read.
    let key = crate::cache::new_key();
    entry
        .set_password(&crate::cache::write_key(&key))
        .map_err(|_| anyhow!("could not save the cache key in the OS credential store"))?;
    Ok(key)
}

/// Forgets the account the service is using: its token, and the account
/// linked before accounts were kept apart, with the key of the files kept
/// for it. Other linked accounts stay.
pub fn forget() -> Result<()> {
    crate::cache::forget();
    set_current(None);
    if crate::client::test_server().is_some() {
        *lock(test_link()) = None;
        lock(test_accounts()).clear();
        return Ok(());
    }
    for account in [ACCOUNT, EARLIER_ACCOUNT, CACHE_KEY] {
        match named(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(_) => bail!("could not remove the token from the OS credential store"),
        }
    }
    Ok(())
}

/// Forgets one account's token, leaving every other linked account alone.
pub fn forget_account(id: &str) -> Result<()> {
    if current_account().as_deref() == Some(id) {
        set_current(None);
    }
    if crate::client::test_server().is_some() {
        lock(test_accounts()).remove(id);
        return Ok(());
    }
    match named(&account_name(id))?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => bail!("could not remove the token from the OS credential store"),
    }
}

/// Accept the header value as copied from browser developer tools, with or
/// without surrounding quotes or whitespace.
pub fn clean(token: &str) -> Result<String> {
    let token = token.trim().trim_matches(['"', '\'']).trim();
    if token.is_empty() || token.contains(char::is_whitespace) {
        bail!("That doesn't look like a Discord token.");
    }
    Ok(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_copied_header_values() {
        assert_eq!(clean("  \"abc.def.ghi\"\n").unwrap(), "abc.def.ghi");
        assert!(clean("   ").is_err());
        assert!(clean("Bearer abc").is_err());
    }

    #[test]
    fn each_account_has_a_name_of_its_own() {
        assert_ne!(account_name("1"), account_name("2"));
        assert_eq!(account_name("1"), "discord-token-1");
    }
}
