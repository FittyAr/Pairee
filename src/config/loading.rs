//! Steps of [`super::AppConfig::load_or_create`]: folders, settings,
//! keybindings, preset keymaps and theme, each created with defaults when
//! missing.

use super::keybindings::KeybindingsConfig;
use super::settings::Settings;
use super::theme::Theme;
use super::{paths, write_atomic};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Creates the config and cache folders; removes legacy JSON translations.
pub(super) fn prepare_dirs() -> Result<()> {
    let config_dir = paths::get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;
    } else {
        remove_legacy_translations(&config_dir.join("lang"));
    }

    let cache_dir = paths::get_cache_dir();
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
    }
    Ok(())
}

/// Clean up legacy JSON translation files from previous versions.
fn remove_legacy_translations(lang_dir: &Path) {
    let Ok(entries) = fs::read_dir(lang_dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let _ = fs::remove_file(path);
        }
    }
}

/// Reads `config.toml` (written with defaults when missing). A parse error
/// yields default settings (keeping the language when possible) and the error.
pub(super) fn load_settings(settings_path: &Path) -> Result<(Settings, Option<String>)> {
    if !settings_path.exists() {
        let default_settings = Settings::default();
        let toml_str = toml::to_string_pretty(&default_settings)
            .context("Failed to serialize default settings")?;
        write_atomic(settings_path, toml_str.as_bytes())
            .context("Failed to write default config.toml")?;
        return Ok((default_settings, None));
    }
    let content = fs::read_to_string(settings_path).context("Failed to read config.toml")?;
    match toml::from_str(&content) {
        Ok(s) => Ok((s, None)),
        Err(e) => {
            // Keep the user's language if the file is still valid TOML
            // so the error below is shown in the language they chose.
            let mut fallback = Settings::default();
            if let Some(lang) = language_of(&content) {
                fallback.language = lang;
            }
            Ok((fallback, Some(e.to_string())))
        }
    }
}

/// The `language` of a `config.toml` text, if it is valid TOML and has one.
fn language_of(content: &str) -> Option<String> {
    content
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| t.get("language")?.as_str().map(str::to_owned))
}

/// The language set in `config.toml`, read without creating or changing
/// any file (the default when the file is missing or unreadable).
pub(super) fn configured_language(settings_path: &Path) -> String {
    fs::read_to_string(settings_path)
        .ok()
        .and_then(|content| language_of(&content))
        .unwrap_or_else(|| Settings::default().language)
}

/// Clears `active_dev_plugin` when its folder or manifest is gone, saving
/// the change when `may_save` (the file parsed).
pub(super) fn drop_missing_dev_plugin(
    settings: &mut Settings,
    settings_path: &Path,
    may_save: bool,
) {
    let Some(ref folder_name) = settings.active_dev_plugin else {
        return;
    };
    let path = if Path::new(folder_name).is_absolute() {
        PathBuf::from(folder_name)
    } else {
        PathBuf::from(&settings.plugins_dev_dir).join(folder_name)
    };
    if path.exists() && path.is_dir() && path.join("manifest.toml").exists() {
        return;
    }
    settings.active_dev_plugin = None;
    if may_save
        && let Ok(toml_str) = super::serialize_settings(settings)
        && let Err(e) = write_atomic(settings_path, toml_str.as_bytes())
    {
        log::warn!(
            "Failed to persist cleared active_dev_plugin to {:?}: {}",
            settings_path,
            e
        );
    }
}

/// A loaded file plus its parse error, when it was invalid (the value is
/// then the default and the file must not be overwritten).
pub(super) type Loaded<T> = (T, Option<String>);

/// Reads `keybindings.toml` (written with defaults when missing). A missing
/// file takes its preset from the legacy `keybinding_preset` key of
/// `config.toml` (`settings_path`), which older releases also wrote. An
/// invalid file yields the defaults and its parse error.
pub(super) fn load_keybindings(settings_path: &Path) -> Result<Loaded<KeybindingsConfig>> {
    let keybindings_path = paths::get_keybindings_file_path();
    if keybindings_path.exists() {
        let content =
            fs::read_to_string(&keybindings_path).context("Failed to read keybindings.toml")?;
        return Ok(match KeybindingsConfig::from_toml(&content) {
            Ok(cfg) => (cfg, None),
            Err(e) => (KeybindingsConfig::default(), Some(e.to_string())),
        });
    }
    let mut default_keybindings = KeybindingsConfig::default();
    if let Some(preset) = fs::read_to_string(settings_path)
        .ok()
        .and_then(|content| legacy_preset(&content))
    {
        default_keybindings.preset = preset;
    }
    let toml_str = toml::to_string_pretty(&default_keybindings)
        .context("Failed to serialize default keybindings")?;
    write_atomic(&keybindings_path, toml_str.as_bytes())
        .context("Failed to write default keybindings.toml")?;
    Ok((default_keybindings, None))
}

/// The `keybinding_preset` key that `config.toml` held before the preset
/// moved to `keybindings.toml`.
fn legacy_preset(config_toml: &str) -> Option<String> {
    let table = config_toml.parse::<toml::Table>().ok()?;
    Some(table.get("keybinding_preset")?.as_str()?.to_owned())
}

/// `true` for a theme name that is a plain file stem: letters, digits,
/// `_`, `-`, `.` and spaces, no leading dot and no `..`, so
/// `themes/<name>.toml` can never point outside the themes folder.
pub fn valid_theme_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
}

/// `themes/<name>.toml` for a [`valid_theme_name`].
pub fn theme_path(theme_name: &str) -> Option<PathBuf> {
    valid_theme_name(theme_name).then(|| paths::get_themes_dir().join(format!("{theme_name}.toml")))
}

/// Reads `themes/<name>.toml`, falling back to the built-in theme of that
/// name; an invalid file yields the built-in theme and its parse error.
pub(super) fn load_theme(theme_name: &str) -> Result<Loaded<Theme>> {
    let builtin = || {
        if theme_name == "classic_blue" {
            Theme::classic_blue()
        } else {
            Theme::default()
        }
    };
    let Some(theme_path) = theme_path(theme_name).filter(|p| p.exists()) else {
        return Ok((builtin(), None));
    };
    let content = fs::read_to_string(&theme_path).context("Failed to read theme file")?;
    Ok(match toml::from_str(&content) {
        Ok(theme) => (theme, None),
        Err(e) => (builtin(), Some(e.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_cannot_leave_the_themes_folder() {
        for ok in ["slate", "classic_blue", "My theme-2", "v1.2"] {
            assert!(valid_theme_name(ok), "{ok}");
        }
        for bad in [
            "",
            "../config",
            "..",
            "a/b",
            "a\\b",
            "C:evil",
            ".hidden",
            "x..y",
        ] {
            assert!(!valid_theme_name(bad), "{bad}");
            assert!(theme_path(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn legacy_preset_is_read_from_old_config_toml() {
        let old = "show_hidden = true
keybinding_preset = \"neovim\"
";
        assert_eq!(legacy_preset(old).as_deref(), Some("neovim"));
        assert_eq!(
            legacy_preset(
                "show_hidden = true
"
            ),
            None
        );
        assert_eq!(legacy_preset("not toml ["), None);
    }
}
