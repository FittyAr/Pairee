//! Connection passwords (and S3 secret keys) live in the system keyring
//! (Windows Credential Manager, macOS Keychain, Secret Service on
//! Linux/BSD), not in `config.toml`.
//!
//! * Each entry is keyed by the connection's unique `id`
//!   (`connection-id:<id>`), so two connections with the same name (a
//!   hand-edited file) never share a password, and a rename keeps it.
//!   Connections without an id get one on load.
//! * Loading fills each connection's password from the keyring. A password
//!   still written in `config.toml` (older releases) is moved into the
//!   keyring and the file is rewritten without it. Entries of older
//!   releases, keyed by name (`connection:<name>`, `ssh-preset:<name>`),
//!   are moved to the id-keyed account.
//! * A keyring that cannot be read (locked, D-Bus hiccup) is not the same
//!   as "no password": the connection is marked `secret_unread` and saving
//!   leaves its entries alone instead of deleting them.
//! * Saving writes a copy of the settings whose passwords were stored in
//!   the keyring (or removed from it when the password was cleared). The
//!   entries of connections that were removed are deleted too: an index
//!   entry in the keyring lists the accounts Pairee wrote (the keyring
//!   itself cannot be listed).
//! * Without a usable keyring, passwords are only written to `config.toml`
//!   when the user opted in (`[security] passwords_in_config = true`);
//!   otherwise they are kept for the session only and a warning is shown.
//! * Passwords held in memory are [`Secret`]s, wiped when dropped.
//!
//! Tests use an in-memory store, never the real keyring.

mod index;
mod store;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use store::test_store;

use super::settings::{Connection, Settings};
use crate::credential_prompt::Secret;
use store::store;

/// Keyring account of a connection's password: by id, or by name for a
/// connection that has no id yet (one created since the last load).
fn account(conn: &Connection) -> String {
    if conn.id.is_empty() {
        name_account(&conn.name)
    } else {
        format!("connection-id:{}", conn.id)
    }
}

/// Account releases before ids kept a connection's password under.
fn name_account(name: &str) -> String {
    format!("connection:{name}")
}

/// Account older releases kept an SSH preset's password under.
fn legacy_account(name: &str) -> String {
    format!("ssh-preset:{name}")
}

/// The accounts of `conn` from older releases, oldest last.
fn old_accounts(conn: &Connection) -> Vec<String> {
    let mut old = vec![name_account(&conn.name), legacy_account(&conn.name)];
    old.retain(|a| *a != account(conn));
    old
}

/// What loading the passwords found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LoadOutcome {
    /// Plain-text passwords were moved to the keyring or connections got
    /// their id: rewrite the file.
    pub migrated: bool,
    /// Plain-text passwords could not be moved (no keyring, no opt-in):
    /// they stay for this session only.
    pub not_persisted: bool,
}

/// Gives every connection a unique id (a copied entry with a repeated id
/// gets a new one). Returns whether any changed.
pub fn assign_ids(settings: &mut Settings) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut changed = false;
    for conn in &mut settings.connections {
        if !conn.id.is_empty() && !seen.insert(conn.id.clone()) {
            conn.id.clear();
        }
        if conn.id.is_empty() {
            conn.ensure_id();
            seen.insert(conn.id.clone());
            changed = true;
        }
    }
    changed
}

/// The stored secret of `conn`: its own entry, else one an older release
/// wrote (copied to the new account; the old one is listed in `moved` to
/// be deleted once every connection was read, since connections sharing a
/// name share it). `Err` when the keyring could not be read.
fn read_secret(conn: &Connection, moved: &mut Vec<String>) -> Result<Option<Secret>, String> {
    if let Some(secret) = store().get(&account(conn))? {
        return Ok(Some(Secret::new(secret)));
    }
    for old in old_accounts(conn) {
        let Some(secret) = store().get(&old)? else {
            continue;
        };
        if store().set(&account(conn), &secret).is_ok() {
            moved.push(old);
        }
        return Ok(Some(Secret::new(secret)));
    }
    Ok(None)
}

/// Fills connection passwords from the keyring and moves plain-text ones
/// into it.
pub fn load_passwords(settings: &mut Settings) -> LoadOutcome {
    let mut outcome = LoadOutcome {
        migrated: assign_ids(settings),
        ..LoadOutcome::default()
    };
    let opt_in = settings.security.passwords_in_config;
    let mut moved = Vec::new();
    for conn in &mut settings.connections {
        match conn.secret() {
            Some(plain) => match store().set(&account(conn), plain) {
                Ok(()) => outcome.migrated = true,
                Err(e) => {
                    tracing::warn!(
                        "Cannot move the password of connection {:?} to the keyring: {e}",
                        conn.name
                    );
                    outcome.not_persisted |= !opt_in;
                }
            },
            None => fill_from_keyring(conn, &mut moved),
        }
    }
    moved.sort();
    moved.dedup();
    for old in moved {
        if let Err(e) = store().delete(&old) {
            tracing::debug!("Old keyring entry {old:?} not removed: {e}");
        }
    }
    if let Err(e) = index::track(settings) {
        tracing::debug!("Connection keyring index not updated: {e}");
    }
    outcome
}

/// Reads the password of `conn` from the keyring; a read error marks it
/// `secret_unread` (so saving keeps the entry).
fn fill_from_keyring(conn: &mut Connection, moved: &mut Vec<String>) {
    match read_secret(conn, moved) {
        Ok(secret) => {
            conn.password = secret;
            conn.secret_unread = false;
        }
        Err(e) => {
            tracing::warn!("Keyring unreadable for connection {:?}: {e}", conn.name);
            conn.secret_unread = true;
        }
    }
}

/// Tries the keyring again for a connection whose password could not be
/// read at startup (called from the background job that connects).
pub fn retry_unread(conn: &mut Connection) {
    if conn.secret_unread && conn.password.is_none() {
        // Old entries are left for the next load to retire.
        fill_from_keyring(conn, &mut Vec::new());
    }
}

/// The settings as they may be written to `config.toml`: passwords go to
/// the keyring (and cleared ones are removed from it); a password the
/// keyring cannot take stays in the file only with the user's opt-in.
pub fn for_disk(settings: &Settings) -> Settings {
    let mut disk = settings.clone();
    let opt_in = settings.security.passwords_in_config;
    for conn in &mut disk.connections {
        store_password(conn, opt_in);
    }
    if let Err(e) = index::prune_removed(settings) {
        tracing::debug!("Passwords of removed connections not pruned: {e}");
    }
    disk
}

/// Moves the password of `conn` (a copy being written to disk) into the
/// keyring, or removes the entry of a password the user cleared.
fn store_password(conn: &mut Connection, opt_in: bool) {
    let account = account(conn);
    let stored = match conn.secret() {
        Some(secret) => store().set(&account, secret),
        // Unknown, not cleared: the entry may hold the real password.
        None if conn.secret_unread => return,
        None => store().delete(&account),
    };
    match stored {
        Ok(()) => conn.password = None,
        Err(e) if conn.password.is_some() && !opt_in => {
            tracing::warn!(
                "Password of connection {:?} not saved: no keyring ({e}) and \
                 [security] passwords_in_config is off",
                conn.name
            );
            conn.password = None;
        }
        Err(e) => tracing::warn!("Keyring unavailable for connection {:?}: {e}", conn.name),
    }
}
