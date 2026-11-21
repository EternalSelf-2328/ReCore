//! The Discord accounts linked on this device, and which one is in use.
//!
//! Each account's token lives in the OS credential store under its Discord
//! user id; only what the window lists, its display name and username, is
//! kept here. `layout.json` keeps both this list and the selected account, so
//! a start connects the right one without asking Discord again.

use serde::{Deserialize, Serialize};

/// One linked Discord account, as Settings lists it. Its token is in the OS
/// credential store, under [`Self::id`], never here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedAccount {
    /// Its Discord user id, which names its token.
    pub id: String,
    /// The name it shows, which is its global name when it set one.
    #[serde(default)]
    pub name: String,
    /// Its username, without the `@`.
    #[serde(default)]
    pub username: String,
}

impl SavedAccount {
    /// The name to show, which is the username until a session names it.
    pub fn label(&self) -> &str {
        if self.name.is_empty() {
            &self.username
        } else {
            &self.name
        }
    }
}

/// The accounts linked on this device, the selected one first in meaning: a
/// switch only changes [`Self::active`], and the selected account's token is
/// the one the Discord service runs with.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordAccounts {
    #[serde(default)]
    pub saved: Vec<SavedAccount>,
    /// The id of the account in use, if any.
    #[serde(default)]
    pub active: Option<String>,
}

/// What removing an account changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Removed {
    /// The account was in the list.
    pub found: bool,
    /// It was the one in use, so another, if one remains, took its place.
    pub was_active: bool,
    /// The account in use afterwards, or none when no account remains.
    pub active: Option<String>,
}

impl DiscordAccounts {
    /// Whether no account is saved.
    pub fn is_empty(&self) -> bool {
        self.saved.is_empty()
    }

    /// The account in use, if it is still listed.
    pub fn active(&self) -> Option<&SavedAccount> {
        let id = self.active.as_deref()?;
        self.saved.iter().find(|account| account.id == id)
    }

    /// Adds an account, or refreshes one already saved, and makes it the one
    /// in use when none is. Returns whether it was new.
    pub fn add(&mut self, account: SavedAccount) -> bool {
        let id = account.id.clone();
        match self.saved.iter_mut().find(|saved| saved.id == id) {
            Some(saved) => {
                if !account.name.is_empty() {
                    saved.name = account.name;
                }
                if !account.username.is_empty() {
                    saved.username = account.username;
                }
                false
            }
            None => {
                self.saved.push(account);
                if self.active.is_none() {
                    self.active = Some(id);
                }
                true
            }
        }
    }

    /// Makes the account with this id the one in use. Returns false when it
    /// isn't saved, or already was.
    pub fn select(&mut self, id: &str) -> bool {
        if self.active.as_deref() == Some(id) || !self.saved.iter().any(|each| each.id == id) {
            return false;
        }
        self.active = Some(id.to_owned());
        true
    }

    /// Removes an account. When it was in use, the first account left takes
    /// its place, so a device with another account stays connected.
    pub fn remove(&mut self, id: &str) -> Removed {
        let found = self.saved.iter().any(|account| account.id == id);
        let was_active = self.active.as_deref() == Some(id);
        self.saved.retain(|account| account.id != id);
        if was_active {
            self.active = self.saved.first().map(|account| account.id.clone());
        }
        Removed {
            found,
            was_active,
            active: self.active.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str, name: &str) -> SavedAccount {
        SavedAccount {
            id: id.into(),
            name: name.into(),
            username: format!("{name}_user"),
        }
    }

    #[test]
    fn a_new_account_is_kept_once_and_the_first_is_used() {
        let mut accounts = DiscordAccounts::default();
        assert!(accounts.add(account("1", "alice")));
        assert_eq!(accounts.active.as_deref(), Some("1"));
        // The same account again updates what it's called, and doesn't
        // become a second row.
        assert!(!accounts.add(account("1", "Alice")));
        assert_eq!(accounts.saved.len(), 1);
        assert_eq!(accounts.saved[0].name, "Alice");
        assert!(accounts.add(account("2", "bob")));
        assert_eq!(accounts.saved.len(), 2);
        assert_eq!(accounts.active.as_deref(), Some("1"), "the first stays");
    }

    #[test]
    fn only_a_saved_account_can_be_selected() {
        let mut accounts = DiscordAccounts::default();
        accounts.add(account("1", "alice"));
        accounts.add(account("2", "bob"));
        assert!(accounts.select("2"));
        assert_eq!(accounts.active().map(|each| each.label()), Some("bob"));
        assert!(!accounts.select("2"), "already selected");
        assert!(!accounts.select("3"), "not linked");
        assert_eq!(accounts.active.as_deref(), Some("2"));
    }

    #[test]
    fn removing_the_active_account_falls_back_to_another() {
        let mut accounts = DiscordAccounts::default();
        accounts.add(account("1", "alice"));
        accounts.add(account("2", "bob"));
        accounts.select("2");
        let removed = accounts.remove("2");
        assert_eq!(
            removed,
            Removed {
                found: true,
                was_active: true,
                active: Some("1".into())
            }
        );
        assert_eq!(accounts.active().map(|each| each.label()), Some("alice"));
    }

    #[test]
    fn removing_another_account_leaves_the_active_one_alone() {
        let mut accounts = DiscordAccounts::default();
        accounts.add(account("1", "alice"));
        accounts.add(account("2", "bob"));
        let removed = accounts.remove("2");
        assert!(removed.found);
        assert!(!removed.was_active);
        assert_eq!(removed.active.as_deref(), Some("1"));
        // Removing the last leaves nothing to use, and nothing to fall back
        // to.
        let removed = accounts.remove("1");
        assert!(removed.was_active);
        assert_eq!(removed.active, None);
        assert!(accounts.is_empty());
        assert!(!accounts.remove("1").found);
    }

    #[test]
    fn an_account_without_a_display_name_shows_its_username() {
        let mut account = account("1", "alice");
        account.name.clear();
        assert_eq!(account.label(), "alice_user");
    }
}
