pub mod associations;
pub mod bookmarks;
pub mod history;
pub mod keybindings;
mod keymap_seed;
pub mod load_guard;
pub mod localization;
pub mod paths;
pub mod session;
pub mod settings;
pub mod theme;
pub mod toml_store;

mod loading;

use anyhow::{Context, Result};
use keybindings::KeybindingsConfig;
use load_guard::ConfigLoadState;
use settings::Settings;
use std::fs;
use theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub settings: Settings,
    pub theme: Theme,
    pub keybindings: KeybindingsConfig,
    /// Whether `config.toml` may be overwritten (locked after a parse error).
    pub load_state: ConfigLoadState,
    /// Localized load error to show once at startup.
    pub load_error: Option<String>,
}

impl AppConfig {
    /// Loads the settings, theme, and keybindings from disk.
    /// If the configuration directory or files do not exist, they are created with default values.
    pub fn load_or_create() -> Result<Self> {
        loading::prepare_dirs()?;

        // 1. Settings Loading
        let settings_path = paths::get_config_file_path();
        let (mut settings, settings_parse_error) = loading::load_settings(&settings_path)?;
        loading::drop_missing_dev_plugin(
            &mut settings,
            &settings_path,
            settings_parse_error.is_none(),
        );

        // Load active language
        localization::load_language(&settings.language);
        let (load_state, load_error) = match settings_parse_error {
            Some(err) => (
                ConfigLoadState::SaveLocked,
                Some(load_guard::backup_invalid_config(&settings_path, &err)),
            ),
            None => (ConfigLoadState::Loaded, None),
        };

        // 2. Keybindings Loading, 2b. preset keymap files
        let keybindings = loading::load_keybindings(&settings_path)?;
        keymap_seed::seed_preset_keymaps(&paths::get_keymaps_dir())?;

        // 3. Theme Loading
        let theme = loading::load_theme(&settings.theme)?;

        Ok(Self {
            settings,
            theme,
            keybindings,
            load_state,
            load_error,
        })
    }

    /// `true` while `config.toml` must not be overwritten implicitly.
    pub fn settings_save_locked(&self) -> bool {
        self.load_state == ConfigLoadState::SaveLocked
    }

    /// Lifts the lock after the user explicitly confirmed saving the settings.
    pub fn confirm_settings_overwrite(&mut self) {
        self.load_state = ConfigLoadState::Loaded;
    }

    /// Returns (once) the localized load error to show at startup, if any.
    pub fn take_load_error(&mut self) -> Option<String> {
        self.load_error.take()
    }

    /// Persists the active configuration back to the disk.
    pub fn save(&self) -> Result<()> {
        let settings_path = paths::get_config_file_path();
        if self.settings_save_locked() {
            log::warn!(
                "Not overwriting invalid {:?} until the user confirms via Save setup",
                settings_path
            );
        } else {
            let settings_toml = toml::to_string_pretty(&self.settings)?;
            write_atomic(&settings_path, settings_toml.as_bytes())?;
        }

        let keybindings_path = paths::get_keybindings_file_path();
        let keybindings_toml = toml::to_string_pretty(&self.keybindings)?;
        write_atomic(&keybindings_path, keybindings_toml.as_bytes())?;

        // Save active theme
        let theme_name = &self.settings.theme;
        let themes_dir = paths::get_themes_dir();
        if !themes_dir.exists() {
            fs::create_dir_all(&themes_dir)?;
        }
        let theme_path = themes_dir.join(format!("{}.toml", theme_name));
        let theme_toml = toml::to_string_pretty(&self.theme)?;
        write_atomic(&theme_path, theme_toml.as_bytes())?;

        Ok(())
    }

    /// Same as [`Self::save`] but never returns an error to the caller:
    /// failures are logged at WARN level instead. Use this for
    /// non-critical write paths (e.g. after a settings change) where a
    /// failed persist should not abort the action the user just
    /// performed — but it must not be silent, otherwise the user
    /// thinks their preference was saved when it was not.
    pub fn save_logging(&self) {
        if let Err(e) = self.save() {
            log::warn!(
                "Failed to persist Pairee configuration; the user's last \
                 setting change may be lost on next launch: {}",
                e
            );
        }
    }
}

/// Atomically replaces `path` with `data` by writing to a unique temp
/// file in the same directory and then renaming it over the target. The
/// rename step is atomic on the same filesystem, so a crash or power
/// loss cannot leave a half-written file behind. Returns the wrapped
/// `io::Error` (or its `Context`) on failure so callers can surface a
/// user-facing message.
///
/// On Unix the file is created with mode `0600`, since config files may
/// hold paths, SSH host names and other private settings.
pub fn write_atomic(path: &std::path::Path, data: &[u8]) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).context("Creating parent directory for atomic write")?;
    }
    let tmp_path = if let Some(parent) = path.parent() {
        let name = format!(
            ".{}.{}.tmp",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("pairee"),
            uuid::Uuid::new_v4()
        );
        parent.join(name)
    } else {
        // No parent — fall back to writing the file in place. This path
        // is only reachable for pathological inputs (a relative
        // `config.toml` with no parent), which the rest of the app does
        // not produce.
        let mut f = create_private_file(path).context("Atomic write fallback (no parent dir)")?;
        return f
            .write_all(data)
            .context("Atomic write fallback (no parent dir)");
    };

    // Write the data to the temp file, fsync to flush to disk, then
    // rename onto the target. If any step fails we try to remove the
    // temp file so we do not leak a `.tmp` next to the config.
    let write_result = (|| -> Result<()> {
        let mut f =
            create_private_file(&tmp_path).context("Creating temp file for atomic write")?;
        f.write_all(data).context("Writing to temp file")?;
        f.sync_all().context("Syncing temp file to disk")?;
        // On Windows `fs::rename` is not atomic when the destination
        // exists. We use `std::fs::rename` which still does the right
        // thing in practice (it falls back to MoveFileEx with
        // MOVEFILE_REPLACE_EXISTING); on POSIX it is a single rename(2)
        // syscall.
        fs::rename(&tmp_path, path).context("Renaming temp file onto target")?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_result
}

/// Creates (or truncates) a file readable/writable only by the owner on Unix.
fn create_private_file(path: &std::path::Path) -> std::io::Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_write_atomic_creates_private_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        write_atomic(&path, b"secret").expect("atomic write");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn save_lock_is_per_config_and_lifted_by_confirmation() {
        let mut locked = AppConfig {
            load_state: ConfigLoadState::SaveLocked,
            load_error: Some("bad".to_string()),
            ..AppConfig::default()
        };
        assert!(locked.settings_save_locked());
        assert!(
            !AppConfig::default().settings_save_locked(),
            "another config instance is unaffected"
        );
        assert_eq!(locked.take_load_error().as_deref(), Some("bad"));
        assert!(locked.take_load_error().is_none());
        locked.confirm_settings_overwrite();
        assert!(!locked.settings_save_locked());
    }

    #[test]
    fn test_write_atomic_replaces_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.toml");
        fs::write(&path, b"original").unwrap();
        write_atomic(&path, b"new content").expect("atomic write");
        let read = fs::read(&path).expect("read back");
        assert_eq!(read, b"new content");
    }

    #[test]
    fn test_write_atomic_creates_missing_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("data.toml");
        write_atomic(&path, b"hi").expect("atomic write to nested path");
        assert_eq!(fs::read(&path).unwrap(), b"hi");
    }

    #[test]
    fn test_write_atomic_does_not_leak_tmp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.toml");
        write_atomic(&path, b"a").unwrap();
        // No `.tmp` leftovers next to the target.
        let stray: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            stray.is_empty(),
            "atomic write left temp file behind: {:?}",
            stray
        );
    }
}
