//! Protects a user's `config.toml` that failed to parse.
//!
//! When the settings file is invalid Pairee starts with defaults, but writing
//! those defaults back would silently destroy the user's (possibly almost
//! correct) configuration. This module backs the file up, records a
//! user-facing error and blocks implicit writes of `config.toml` until the
//! user explicitly confirms a save (the "Save setup" dialog).

use super::localization::t;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static SETTINGS_WRITE_LOCKED: AtomicBool = AtomicBool::new(false);
static LOAD_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Path of the backup created next to `path` (`config.toml` → `config.toml.bak`).
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// Handles a settings parse failure: backs the file up, locks implicit saves
/// and stores a message for [`take_load_error`]. Call after the UI language
/// has been loaded so the message is localized.
pub fn record_parse_failure(path: &Path, parse_error: &str) {
    let bak = backup_path(path);
    let backup_result = std::fs::copy(path, &bak);
    SETTINGS_WRITE_LOCKED.store(true, Ordering::SeqCst);

    let message = match backup_result {
        Ok(_) => {
            tracing::error!(
                "config.toml at {:?} is invalid ({parse_error}); backed up to {:?}, using defaults",
                path,
                bak
            );
            t("error_config_invalid")
                .replacen("{}", &path.display().to_string(), 1)
                .replacen("{}", parse_error.trim(), 1)
                .replacen("{}", &bak.display().to_string(), 1)
        }
        Err(e) => {
            tracing::error!(
                "config.toml at {:?} is invalid ({parse_error}); backup to {:?} failed: {e}",
                path,
                bak
            );
            t("error_config_invalid_no_backup")
                .replacen("{}", &path.display().to_string(), 1)
                .replacen("{}", parse_error.trim(), 1)
                .replacen("{}", &e.to_string(), 1)
        }
    };
    if let Ok(mut slot) = LOAD_ERROR.lock() {
        *slot = Some(message);
    }
}

/// Returns (once) the localized load error to show at startup, if any.
pub fn take_load_error() -> Option<String> {
    LOAD_ERROR.lock().ok().and_then(|mut slot| slot.take())
}

/// `true` while `config.toml` must not be overwritten implicitly.
pub fn settings_write_locked() -> bool {
    SETTINGS_WRITE_LOCKED.load(Ordering::SeqCst)
}

/// Lifts the lock after the user explicitly confirmed saving the settings.
pub fn confirm_settings_overwrite() {
    SETTINGS_WRITE_LOCKED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_path_appends_bak() {
        let p = Path::new("/tmp/pairee/config.toml");
        assert_eq!(backup_path(p), Path::new("/tmp/pairee/config.toml.bak"));
    }

    #[test]
    fn parse_failure_backs_up_and_locks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "theme = [broken").unwrap();

        record_parse_failure(&path, "expected value");
        assert_eq!(
            std::fs::read_to_string(backup_path(&path)).unwrap(),
            "theme = [broken"
        );
        assert!(settings_write_locked());
        assert!(take_load_error().is_some());
        assert!(take_load_error().is_none());

        confirm_settings_overwrite();
        assert!(!settings_write_locked());
    }
}
