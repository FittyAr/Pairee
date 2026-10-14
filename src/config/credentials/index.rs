//! The keyring cannot be listed, so an index entry lists the accounts
//! Pairee may have written; the entries of connections that were removed
//! are found there and deleted. Older releases kept indexes of connection
//! names (`connection-index`) and of SSH presets (`ssh-preset-index`),
//! which are folded in once.

use super::store::store;
use super::{account, legacy_account, name_account, old_accounts};
use crate::config::settings::Settings;

/// Keyring account holding the accounts that may have a password entry
/// (a JSON list).
pub(super) const INDEX_ACCOUNT: &str = "connection-accounts";
/// The index of connection names of releases before ids.
pub(super) const NAME_INDEX_ACCOUNT: &str = "connection-index";
/// The index of older releases (SSH presets only).
pub(super) const LEGACY_INDEX_ACCOUNT: &str = "ssh-preset-index";

fn read(account: &str) -> Result<Vec<String>, String> {
    Ok(store()
        .get(account)?
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default())
}

fn write(accounts: &[String]) -> Result<(), String> {
    if accounts.is_empty() {
        return store().delete(INDEX_ACCOUNT);
    }
    let text = serde_json::to_string(accounts).map_err(|e| e.to_string())?;
    store().set(INDEX_ACCOUNT, &text)
}

/// The accounts the current connections use. A connection whose password
/// could not be read keeps its older accounts too (the password may still
/// be there only).
fn current(settings: &Settings) -> Vec<String> {
    let mut accounts: Vec<String> = Vec::new();
    for conn in &settings.connections {
        accounts.push(account(conn));
        if conn.secret_unread {
            accounts.extend(old_accounts(conn));
        }
    }
    accounts.sort();
    accounts.dedup();
    accounts
}

/// Adds the loaded connections' accounts to the index, so entries written
/// before the index existed are found once those connections are removed,
/// and folds in the indexes of older releases.
pub(super) fn track(settings: &Settings) -> Result<(), String> {
    let mut index = read(INDEX_ACCOUNT)?;
    let before = index.clone();
    index.extend(current(settings));
    let names = read(NAME_INDEX_ACCOUNT)?;
    index.extend(names.iter().map(|name| name_account(name)));
    retire_legacy(settings, &mut index)?;
    index.sort();
    index.dedup();
    if index != before {
        write(&index)?;
    }
    if !names.is_empty() {
        store().delete(NAME_INDEX_ACCOUNT)?;
    }
    Ok(())
}

/// Deletes the `ssh-preset:` entries listed by the oldest index (those of
/// current connections were moved to their new account while loading)
/// and then that index itself. An entry whose connection could not be
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
        // A stale name-keyed entry of a removed preset is pruned later.
        let by_name = name_account(&name);
        if store().get(&by_name)?.is_some() {
            index.push(by_name);
        }
    }
    if complete {
        store().delete(LEGACY_INDEX_ACCOUNT)?;
    }
    Ok(())
}

/// Deletes the entries of connections no longer in `settings`. One that
/// cannot be deleted stays in the index for next time.
pub(super) fn prune_removed(settings: &Settings) -> Result<(), String> {
    let current = current(settings);
    let mut index = current.clone();
    for stale in read(INDEX_ACCOUNT)?
        .into_iter()
        .filter(|a| !current.contains(a))
    {
        if let Err(e) = store().delete(&stale) {
            tracing::warn!("Cannot delete the keyring entry {stale:?} of a removed connection: {e}");
            index.push(stale);
        }
    }
    index.sort();
    write(&index)
}
