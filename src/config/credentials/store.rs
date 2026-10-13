//! Where secrets are kept: the system keyring, or an in-memory store in
//! tests (which can simulate a keyring that is unavailable or that fails
//! to read while still answering "no entry" correctly).

/// A place to keep secrets by account name.
///
/// `get` tells "there is no secret" (`Ok(None)`) apart from "the store
/// could not be asked" (`Err`): only the first may ever lead to deleting
/// anything.
pub(super) trait SecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// Keyring service name of Pairee's secrets.
#[cfg(not(test))]
const SERVICE: &str = "pairee";

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
pub(super) fn store() -> &'static dyn SecretStore {
    &OsKeyring
}

#[cfg(test)]
pub(super) fn store() -> &'static dyn SecretStore {
    &test_store::Memory
}

#[cfg(test)]
pub(crate) mod test_store {
    //! Per-thread in-memory keyring for tests.
    use super::SecretStore;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SECRETS: RefCell<HashMap<String, String>> = RefCell::default();
        static AVAILABLE: RefCell<bool> = const { RefCell::new(true) };
        static READS_FAIL: RefCell<bool> = const { RefCell::new(false) };
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
            if READS_FAIL.with(|f| *f.borrow()) {
                return Err("keyring locked".into());
            }
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

    /// `false`: every call fails (no keyring at all).
    pub fn set_available(available: bool) {
        AVAILABLE.with(|a| *a.borrow_mut() = available);
    }

    /// `true`: reads fail (a transient error) while writes still work.
    pub fn set_reads_fail(fail: bool) {
        READS_FAIL.with(|f| *f.borrow_mut() = fail);
    }

    pub fn secret(account: &str) -> Option<String> {
        SECRETS.with(|s| s.borrow().get(account).cloned())
    }

    pub fn put(account: &str, secret: &str) {
        SECRETS.with(|s| s.borrow_mut().insert(account.into(), secret.into()));
    }
}
