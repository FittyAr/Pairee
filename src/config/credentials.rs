//! SSH preset passwords live in the system keyring (Windows Credential
//! Manager, macOS Keychain, Secret Service on Linux/BSD), not in
//! `config.toml`.
//!
//! * Loading fills each preset's password from the keyring. A password
//!   still written in `config.toml` (older releases) is moved into the
//!   keyring and the file is rewritten without it.
//! * Saving writes a copy of the settings whose passwords were stored in
//!   the keyring (or removed from it when the password was cleared).
//! * Without a usable keyring, passwords are only written to `config.toml`
//!   when the user opted in (`[security] ssh_passwords_in_config = true`);
//!   otherwise they are kept for the session only and a warning is shown.
//!
//! Tests use an in-memory store, never the real keyring.

use super::settings::{Settings, SshPreset};

/// Keyring service name of Pairee's secrets.
#[cfg(not(test))]
const SERVICE: &str = "pairee";

/// A place to keep secrets by account name.
trait SecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

#[cfg(not(test))]
struct OsKeyring;

#[cfg(not(test))]
impl OsKeyring {
    fn entry(account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())
    }
}

#[cfg(not(test))]
impl SecretStore for OsKeyring {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        match Self::entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        Self::entry(account)?
            .set_password(secret)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(test))]
fn store() -> &'static dyn SecretStore {
    &OsKeyring
}

#[cfg(test)]
pub(crate) mod test_store {
    //! Per-thread in-memory keyring for tests (can simulate "unavailable").
    use super::SecretStore;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SECRETS: RefCell<HashMap<String, String>> = RefCell::default();
        static AVAILABLE: RefCell<bool> = const { RefCell::new(true) };
    }

    pub struct Memory;

    fn check() -> Result<(), String> {
        if AVAILABLE.with(|a| *a.borrow()) {
            Ok(())
        } else {
            Err("no keyring".into())
        }
    }

    impl SecretStore for Memory {
        fn get(&self, account: &str) -> Result<Option<String>, String> {
            check()?;
            Ok(SECRETS.with(|s| s.borrow().get(account).cloned()))
        }
        fn set(&self, account: &str, secret: &str) -> Result<(), String> {
            check()?;
            SECRETS.with(|s| s.borrow_mut().insert(account.into(), secret.into()));
            Ok(())
        }
        fn delete(&self, account: &str) -> Result<(), String> {
            check()?;
            SECRETS.with(|s| s.borrow_mut().remove(account));
            Ok(())
        }
    }

    pub fn set_available(available: bool) {
        AVAILABLE.with(|a| *a.borrow_mut() = available);
    }

    pub fn secret(account: &str) -> Option<String> {
        SECRETS.with(|s| s.borrow().get(account).cloned())
    }
}

#[cfg(test)]
fn store() -> &'static dyn SecretStore {
    &test_store::Memory
}

/// Keyring account of a preset's password.
fn account(preset: &SshPreset) -> String {
    format!("ssh-preset:{}", preset.name)
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

/// Fills preset passwords from the keyring and moves plain-text ones into it.
pub fn load_passwords(settings: &mut Settings) -> LoadOutcome {
    let mut outcome = LoadOutcome::default();
    let opt_in = settings.security.ssh_passwords_in_config;
    for preset in &mut settings.ssh_presets {
        let account = account(preset);
        match &preset.password {
            Some(plain) => match store().set(&account, plain) {
                Ok(()) => outcome.migrated = true,
                Err(e) => {
                    tracing::warn!(
                        "Cannot move the password of SSH preset {:?} to the keyring: {e}",
                        preset.name
                    );
                    outcome.not_persisted |= !opt_in;
                }
            },
            None => match store().get(&account) {
                Ok(secret) => preset.password = secret,
                Err(e) => {
                    tracing::warn!("Keyring unavailable for SSH preset {:?}: {e}", preset.name)
                }
            },
        }
    }
    outcome
}

/// The settings as they may be written to `config.toml`: passwords go to
/// the keyring (and cleared ones are removed from it); a password the
/// keyring cannot take stays in the file only with the user's opt-in.
pub fn for_disk(settings: &Settings) -> Settings {
    let mut disk = settings.clone();
    let opt_in = settings.security.ssh_passwords_in_config;
    for preset in &mut disk.ssh_presets {
        let account = account(preset);
        let stored = match &preset.password {
            Some(secret) => store().set(&account, secret),
            None => store().delete(&account),
        };
        match stored {
            Ok(()) => preset.password = None,
            Err(e) if preset.password.is_some() && !opt_in => {
                tracing::warn!(
                    "Password of SSH preset {:?} not saved: no keyring ({e}) and \
                     [security] ssh_passwords_in_config is off",
                    preset.name
                );
                preset.password = None;
            }
            Err(e) => tracing::warn!("Keyring unavailable for SSH preset {:?}: {e}", preset.name),
        }
    }
    disk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(password: Option<&str>) -> Settings {
        let mut settings = Settings::default();
        settings.ssh_presets.push(SshPreset {
            name: "box".into(),
            host: "h".into(),
            port: "22".into(),
            username: "u".into(),
            password: password.map(str::to_owned),
            key_path: None,
        });
        settings
    }

    #[test]
    fn plaintext_passwords_migrate_to_the_keyring() {
        test_store::set_available(true);
        let mut settings = settings_with(Some("s3cret"));
        let outcome = load_passwords(&mut settings);
        assert!(outcome.migrated && !outcome.not_persisted);
        assert_eq!(
            test_store::secret("ssh-preset:box").as_deref(),
            Some("s3cret")
        );
        assert_eq!(settings.ssh_presets[0].password.as_deref(), Some("s3cret"));
        let disk = for_disk(&settings);
        assert!(
            disk.ssh_presets[0].password.is_none(),
            "never written to the file"
        );
        let toml = toml::to_string(&disk).unwrap();
        assert!(!toml.contains("s3cret"));
    }

    #[test]
    fn passwords_are_read_back_and_cleared_ones_deleted() {
        test_store::set_available(true);
        let saved = for_disk(&settings_with(Some("pw")));
        let mut loaded = saved.clone();
        assert_eq!(load_passwords(&mut loaded), LoadOutcome::default());
        assert_eq!(loaded.ssh_presets[0].password.as_deref(), Some("pw"));
        let _ = for_disk(&settings_with(None));
        assert!(test_store::secret("ssh-preset:box").is_none());
    }

    #[test]
    fn without_keyring_plaintext_needs_the_opt_in() {
        test_store::set_available(false);
        let mut settings = settings_with(Some("pw"));
        let outcome = load_passwords(&mut settings);
        assert!(outcome.not_persisted);
        assert!(for_disk(&settings).ssh_presets[0].password.is_none());

        settings.security.ssh_passwords_in_config = true;
        assert!(!load_passwords(&mut settings).not_persisted);
        assert_eq!(
            for_disk(&settings).ssh_presets[0].password.as_deref(),
            Some("pw")
        );
        test_store::set_available(true);
    }
}
