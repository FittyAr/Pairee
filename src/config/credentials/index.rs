//! The keyring cannot be listed, so an index entry names the connections
//! that may have a password entry; entries of connections that were
//! removed or renamed are found there and deleted. Older releases kept an
//! index of SSH presets (`ssh-preset-index`), which is folded in once.

use super::store::store;
use super::{account, legacy_account};
use crate::config::settings::Settings;

/// Keyring account holding the names of the connections that may have a
/// password entry (a JSON list).
pub(super) const INDEX_ACCOUNT: &str = "connection-index";
/// The index of older releases (SSH presets only).
pub(super) const LEGACY_INDEX_ACCOUNT: &str = "ssh-preset-index";

fn read(account: &str) -> Result<Vec<String>, String> {
    Ok(store()
        .get(account)?
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default())
}

fn write(names: &[String]) -> Result<(), String> {
    if names.is_empty() {
        return store().delete(INDEX_ACCOUNT);
    }
    let text = serde_json::to_string(names).map_err(|e| e.to_string())?;
    store().set(INDEX_ACCOUNT, &text)
}

fn names(settings: &Settings) -> Vec<String> {
    let mut names: Vec<String> = settings
        .connections
        .iter()
        .map(|c| c.name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Adds the loaded connections to the index, so entries written before the
/// index existed are found once those connections are removed, and folds
/// in the index of older releases.
pub(super) fn track(settings: &Settings) -> Result<(), String> {
    let mut index = read(INDEX_ACCOUNT)?;
    let before = index.clone();
    for name in names(settings) {
        if !index.contains(&name) {
            index.push(name);
        }
    }
    retire_legacy(settings, &mut index)?;
    index.sort();
    index.dedup();
    if index == before {
        return Ok(());
    }
    write(&index)
}

/// Deletes the `ssh-preset:` entries listed by the old index (those of
/// current connections were moved to their new account while loading)
/// and then the old index itself. An entry whose connection could not be
/// read, or that cannot be deleted, keeps the old index for next time.
fn retire_legacy(settings: &Settings, index: &mut Vec<String>) -> Result<(), String> {
    let legacy = read(LEGACY_INDEX_ACCOUNT)?;
    if legacy.is_empty() {
        return Ok(());
    }
    let mut complete = true;
    for name in legacy {
        let unread = settings
            .connections
            .iter()
            .any(|c| c.name == name && c.secret_unread);
        if unread || store().delete(&legacy_account(&name)).is_err() {
            complete = false;
            continue;
        }
        // A stale new-style entry of a removed preset is pruned later.
        if !index.contains(&name) && store().get(&account(&name))?.is_some() {
            index.push(name);
        }
    }
    if complete {
        store().delete(LEGACY_INDEX_ACCOUNT)?;
    }
    Ok(())
}

/// Deletes the entries of connections no longer in `settings` (removed or
/// renamed). One that cannot be deleted stays in the index for next time.
pub(super) fn prune_removed(settings: &Settings) -> Result<(), String> {
    let current = names(settings);
    let mut index = current.clone();
    for stale in read(INDEX_ACCOUNT)?
        .into_iter()
        .filter(|n| !current.contains(n))
    {
        if let Err(e) = store().delete(&account(&stale)) {
            tracing::warn!("Cannot delete the password of removed connection {stale:?}: {e}");
            index.push(stale);
        }
    }
    index.sort();
    write(&index)
}
