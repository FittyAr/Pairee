//! Connection passwords (and S3 secret keys) live in the system keyring
//! (Windows Credential Manager, macOS Keychain, Secret Service on
//! Linux/BSD), not in `config.toml`.
//!
//! * Loading fills each connection's password from the keyring. A password
//!   still written in `config.toml` (older releases) is moved into the
//!   keyring and the file is rewritten without it. Entries of older
//!   releases (`ssh-preset:<name>`) are moved to `connection:<name>`.
//! * A keyring that cannot be read (locked, D-Bus hiccup) is not the same
//!   as "no password": the connection is marked `secret_unread` and saving
//!   leaves its entry alone instead of deleting it.
//! * Saving writes a copy of the settings whose passwords were stored in
//!   the keyring (or removed from it when the password was cleared). The
//!   entries of connections that were removed or renamed are deleted too:
//!   an index entry in the keyring lists the connections that may have one
//!   (the keyring itself cannot be listed).
//! * Without a usable keyring, passwords are only written to `config.toml`
//!   when the user opted in (`[security] passwords_in_config = true`);
//!   otherwise they are kept for the session only and a warning is shown.
//!
//! Tests use an in-memory store, never the real keyring.

mod index;
mod store;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use store::test_store;

use super::settings::{Connection, Settings};
use store::store;

/// Keyring account of a connection's password.
fn account(name: &str) -> String {
    format!("connection:{name}")
}

/// Account older releases kept an SSH preset's password under.
fn legacy_account(name: &str) -> String {
    format!("ssh-preset:{name}")
}

/// What loading the passwords found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LoadOutcome {
    /// Plain-text passwords were moved to the keyring: rewrite the file.
    pub migrated: bool,
    /// Plain-text passwords could not be moved (no keyring, no opt-in):
    /// they stay for this session only.
    pub not_persisted: bool,
}

/// The stored secret of connection `name`: its own entry, else one an
/// older release wrote (moved to the new account on the way). `Err` when
/// the keyring could not be read.
fn read_secret(name: &str) -> Result<Option<String>, String> {
    if let Some(secret) = store().get(&account(name))? {
        return Ok(Some(secret));
    }
    let legacy = legacy_account(name);
    let Some(secret) = store().get(&legacy)? else {
        return Ok(None);
    };
    if store().set(&account(name), &secret).is_ok()
        && let Err(e) = store().delete(&legacy)
    {
        tracing::debug!("Old keyring entry of connection {name:?} not removed: {e}");
    }
    Ok(Some(secret))
}

/// Fills connection passwords from the keyring and moves plain-text ones
/// into it.
pub fn load_passwords(settings: &mut Settings) -> LoadOutcome {
    let mut outcome = LoadOutcome::default();
    let opt_in = settings.security.passwords_in_config;
    for conn in &mut settings.connections {
        match &conn.password {
            Some(plain) => match store().set(&account(&conn.name), plain) {
                Ok(()) => outcome.migrated = true,
                Err(e) => {
                    tracing::warn!(
                        "Cannot move the password of connection {:?} to the keyring: {e}",
                        conn.name
                    );
                    outcome.not_persisted |= !opt_in;
                }
            },
            None => fill_from_keyring(conn),
        }
    }
    if let Err(e) = index::track(settings) {
        tracing::debug!("Connection keyring index not updated: {e}");
    }
    outcome
}

/// Reads the password of `conn` from the keyring; a read error marks it
/// `secret_unread` (so saving keeps the entry).
fn fill_from_keyring(conn: &mut Connection) {
    match read_secret(&conn.name) {
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
        fill_from_keyring(conn);
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
    let account = account(&conn.name);
    let stored = match &conn.password {
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
