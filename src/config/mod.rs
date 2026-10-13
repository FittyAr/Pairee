pub mod associations;
pub mod bookmarks;
pub mod credentials;
pub mod frecency;
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
    /// Same guard for `keybindings.toml`.
    pub keybindings_state: ConfigLoadState,
    /// Same guard for the active `themes/<name>.toml`.
    pub theme_state: ConfigLoadState,
    /// Localized load error to show once at startup.
    pub load_error: Option<String>,
}

/// Loads the language set in `config.toml` without touching any file, for
/// the command-line subcommands that print text but need no configuration.
pub fn load_configured_language() {
    localization::load_language(&loading::configured_language(&paths::get_config_file_path()));
}

impl AppConfig {
    /// Loads the settings, theme, and keybindings from disk.
    /// If the configuration directory or files do not exist, they are created with default values.
    pub fn load_or_create() -> Result<Self> {
        loading::prepare_dirs()?;

        // 1. Settings Loading
        let settings_path = paths::get_config_file_path();
        let (mut settings, settings_parse_error) = loading::load_settings(&settings_path)?;

        // Load active language
        localization::load_language(&settings.language);
        let mut errors = Vec::new();
        let settings_failed = settings_parse_error.is_some();
        let load_state = load_guard::guard(&settings_path, settings_parse_error, &mut errors);
        if settings_failed {
            // Plugins run in Secure Mode until the file is readable again.
            errors.push(localization::t("plugin_secure_mode_forced"));
        }
        let credentials = credentials::load_passwords(&mut settings);
        if credentials.migrated && !settings_failed {
            // Rewrite config.toml without the passwords now in the keyring.
            let migrated = serialize_settings(&settings)
                .and_then(|text| write_atomic(&settings_path, text.as_bytes()));
            if let Err(e) = migrated {
                log::warn!("Could not remove migrated passwords from config.toml: {e:#}");
            }
        }
        if credentials.not_persisted {
            errors.push(localization::t("ssh_password_not_persisted"));
        }
        // After the passwords are loaded: saving settings whose connections have
        // no password yet would remove them from the keyring.
        loading::drop_missing_dev_plugin(&mut settings, &settings_path, !settings_failed);

        // 2. Keybindings Loading, 2b. preset keymap files
        let (keybindings, keybindings_error) = loading::load_keybindings(&settings_path)?;
        let keybindings_state = load_guard::guard(
            &paths::get_keybindings_file_path(),
            keybindings_error,
            &mut errors,
        );
        keymap_seed::seed_preset_keymaps(&paths::get_keymaps_dir())?;

        // 3. Theme Loading (the name becomes a file path: sanitize it)
        if !loading::valid_theme_name(&settings.theme) {
            errors.push(localization::t("error_theme_name_invalid").replacen(
                "{}",
                &settings.theme,
                1,
            ));
            settings.theme = Settings::default().theme;
        }
        let (theme, theme_error) = loading::load_theme(&settings.theme)?;
        let theme_state = match loading::theme_path(&settings.theme) {
            Some(path) => load_guard::guard(&path, theme_error, &mut errors),
            None => ConfigLoadState::Loaded,
        };

        // 4. User syntax definitions (an invalid file is skipped and reported)
        errors.extend(
            crate::syntax::load_user_definitions(&paths::get_syntax_dir())
                .into_iter()
                .map(|(path, error)| {
                    localization::t("error_syntax_file_invalid")
                        .replacen("{}", &path.display().to_string(), 1)
                        .replacen("{}", &error, 1)
                }),
        );

        Ok(Self {
            settings,
            theme,
            keybindings,
            load_state,
            keybindings_state,
            theme_state,
            load_error: (!errors.is_empty()).then(|| errors.join("\n\n")),
        })
    }

    /// Switches to theme `name`: its file in the themes folder when there
    /// is one (an invalid file keeps the built-in look and is not
    /// overwritten), else the built-in theme. Returns the load error to
    /// show, if any.
    pub fn switch_theme(&mut self, name: &str) -> Option<String> {
        if !loading::valid_theme_name(name) {
            return Some(localization::t("error_theme_name_invalid").replacen("{}", name, 1));
        }
        let (theme, error) = match loading::load_theme(name) {
            Ok(loaded) => loaded,
            Err(e) => (theme::Theme::default(), Some(e.to_string())),
        };
        let mut errors = Vec::new();
        self.theme_state = match loading::theme_path(name) {
            Some(path) => load_guard::guard(&path, error, &mut errors),
            None => ConfigLoadState::Loaded,
        };
        self.settings.theme = name.to_string();
        self.theme = theme;
        errors.pop()
    }

    /// `true` while `config.toml` must not be overwritten implicitly.
    pub fn settings_save_locked(&self) -> bool {
        self.load_state == ConfigLoadState::SaveLocked
    }

    /// Lifts the locks after the user explicitly confirmed saving the
    /// settings ("Save setup" replaces every invalid file, already backed up).
    pub fn confirm_settings_overwrite(&mut self) {
        self.load_state = ConfigLoadState::Loaded;
        self.keybindings_state = ConfigLoadState::Loaded;
        self.theme_state = ConfigLoadState::Loaded;
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
            let settings_toml = serialize_settings(&self.settings)?;
            write_atomic(&settings_path, settings_toml.as_bytes())?;
        }

        let keybindings_path = paths::get_keybindings_file_path();
        if self.keybindings_state == ConfigLoadState::SaveLocked {
            log::warn!("Not overwriting invalid {keybindings_path:?} until Save setup");
        } else {
            let keybindings_toml = toml::to_string_pretty(&self.keybindings)?;
            write_atomic(&keybindings_path, keybindings_toml.as_bytes())?;
        }

        // Save active theme (never to a path outside the themes folder)
        match loading::theme_path(&self.settings.theme) {
            Some(theme_path) if self.theme_state == ConfigLoadState::Loaded => {
                let theme_toml = toml::to_string_pretty(&self.theme)?;
                write_atomic(&theme_path, theme_toml.as_bytes())?;
            }
            Some(theme_path) => {
                log::warn!("Not overwriting invalid {theme_path:?} until Save setup");
            }
            None => log::warn!(
                "Not saving theme with invalid name {:?}",
                self.settings.theme
            ),
        }

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

/// `config.toml` text for `settings`: connection passwords go to the keyring
/// (see [`credentials::for_disk`]), never in plain text unless opted in.
pub fn serialize_settings(settings: &Settings) -> Result<String> {
    Ok(toml::to_string_pretty(&credentials::for_disk(settings))?)
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
    fn clearing_a_missing_dev_plugin_keeps_keyring_passwords() {
        let dir = tempfile::tempdir().unwrap();
        let _root = paths::test_root::redirect(dir.path());
        credentials::test_store::set_available(true);
        let mut settings = Settings::default();
        settings.connections.push(settings::Connection {
            name: "box".into(),
            host: "h".into(),
            port: "22".into(),
            username: "u".into(),
            password: Some("pw".into()),
            ..settings::Connection::default()
        });
        settings.active_dev_plugin = Some("no-such-plugin".into());
        write_atomic(
            &paths::get_config_file_path(),
            serialize_settings(&settings).unwrap().as_bytes(),
        )
        .unwrap();

        let config = AppConfig::load_or_create().unwrap();
        assert!(config.settings.active_dev_plugin.is_none());
        assert_eq!(
            config.settings.connections[0].password.as_deref(),
            Some("pw")
        );
        assert_eq!(
            credentials::test_store::secret("connection:box").as_deref(),
            Some("pw")
        );
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
