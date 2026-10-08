//! Protects a user's `config.toml` that failed to parse.
//!
//! When the settings file is invalid Pairee starts with defaults, but writing
//! those defaults back would silently destroy the user's (possibly almost
//! correct) configuration. This module backs the file up and produces a
//! user-facing error; [`ConfigLoadState`] (kept in `AppConfig`) blocks
//! implicit writes of `config.toml` until the user explicitly confirms a save
//! (the "Save setup" dialog).

use super::localization::t;
use std::path::{Path, PathBuf};

/// Whether `config.toml` may be overwritten by implicit saves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConfigLoadState {
    /// The file was read (or created) normally.
    #[default]
    Loaded,
    /// The file could not be parsed and was backed up; it is not overwritten
    /// until the user confirms via "Save setup".
    SaveLocked,
}

/// Path of the backup created next to `path` (`config.toml` → `config.toml.bak`).
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// Handles a settings parse failure: backs the file up and returns the
/// localized message to show at startup. Call after the UI language has
/// been loaded so the message is localized.
pub fn backup_invalid_config(path: &Path, parse_error: &str) -> String {
    let bak = backup_path(path);
    match std::fs::copy(path, &bak) {
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
    }
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
    fn parse_failure_backs_up_and_reports() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "theme = [broken").unwrap();

        let message = backup_invalid_config(&path, "expected value");
        assert!(!message.is_empty());
        assert_eq!(
            std::fs::read_to_string(backup_path(&path)).unwrap(),
            "theme = [broken"
        );
    }
}
