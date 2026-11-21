//! The data folder's `layout.json`, where the window keeps its
//! arrangement and the settings that are the window's own.

use std::{io, path::Path};

/// The file, in the data folder.
pub const LAYOUT: &str = "layout.json";

/// Whether `layout.json` names a linked Discord account, so a build without
/// Matrix can start on it. The list and the one in use are the window's; the
/// tokens are in the OS credential store.
pub fn has_discord_account(root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(root.join(LAYOUT)) else {
        return false;
    };
    let Ok(layout) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    layout["discord_account"]
        .as_str()
        .is_some_and(|id| !id.is_empty())
        || layout["discord_accounts"]
            .as_array()
            .is_some_and(|accounts| !accounts.is_empty())
}

/// Writes `bytes` as `root`'s `layout.json`: to a file beside it, then over
/// it, so a crash or a power cut mid-write leaves the old file whole
/// rather than a short one that reads as empty.
pub fn write(root: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::create_dir_all(root)?;
    let beside = root.join("layout.json.tmp");
    std::fs::write(&beside, bytes)?;
    std::fs::rename(&beside, root.join(LAYOUT))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_layout_is_replaced_whole() {
        let folder = tempfile::tempdir().expect("a folder");
        super::write(folder.path(), b"{\"a\": 1}").expect("written");
        super::write(folder.path(), b"{\"a\": 2}").expect("written again");
        let kept = std::fs::read(folder.path().join(super::LAYOUT)).expect("kept");
        assert_eq!(kept, b"{\"a\": 2}");
        assert!(!folder.path().join("layout.json.tmp").exists());
    }

    #[test]
    fn a_saved_discord_account_is_found_in_the_layout() {
        let folder = tempfile::tempdir().expect("a folder");
        assert!(!super::has_discord_account(folder.path()));
        super::write(
            folder.path(),
            br#"{"discord_accounts":[{"id":"1","name":"Alice","username":"alice"}]}"#,
        )
        .expect("written");
        assert!(super::has_discord_account(folder.path()));
        super::write(folder.path(), br#"{"discord_account":"1"}"#).expect("written");
        assert!(super::has_discord_account(folder.path()));
        super::write(folder.path(), br#"{"discord_accounts":[]}"#).expect("written");
        assert!(!super::has_discord_account(folder.path()));
        super::write(folder.path(), b"not json").expect("written");
        assert!(!super::has_discord_account(folder.path()));
    }
}
