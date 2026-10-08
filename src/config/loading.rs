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
            if let Some(lang) = content
                .parse::<toml::Table>()
                .ok()
                .and_then(|t| t.get("language")?.as_str().map(str::to_owned))
            {
                fallback.language = lang;
            }
            Ok((fallback, Some(e.to_string())))
        }
    }
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
        && let Ok(toml_str) = toml::to_string_pretty(&*settings)
        && let Err(e) = write_atomic(settings_path, toml_str.as_bytes())
    {
        log::warn!(
            "Failed to persist cleared active_dev_plugin to {:?}: {}",
            settings_path,
            e
        );
    }
}

/// Reads `keybindings.toml` (written with defaults when missing).
pub(super) fn load_keybindings() -> Result<KeybindingsConfig> {
    let keybindings_path = paths::get_keybindings_file_path();
    if keybindings_path.exists() {
        let content =
            fs::read_to_string(&keybindings_path).context("Failed to read keybindings.toml")?;
        return Ok(toml::from_str(&content).unwrap_or_default());
    }
    let default_keybindings = KeybindingsConfig::default();
    let toml_str = toml::to_string_pretty(&default_keybindings)
        .context("Failed to serialize default keybindings")?;
    fs::write(&keybindings_path, toml_str).context("Failed to write default keybindings.toml")?;
    Ok(default_keybindings)
}

/// Seeds the `keymaps/` folder with the built-in presets and refreshes
/// copies written by versions with the old F7/F11 layout.
pub(super) fn seed_preset_keymaps(presets: &[&str]) -> Result<()> {
    let keymaps_dir = paths::get_keymaps_dir();
    if !keymaps_dir.exists() {
        fs::create_dir_all(&keymaps_dir).context("Failed to create keymaps directory")?;
    }
    for preset_name in presets {
        let preset_path = keymaps_dir.join(format!("{}.toml", preset_name));
        let toml_content = crate::keybindings::preset::get_builtin_preset_toml(preset_name);
        if !preset_path.exists() {
            fs::write(&preset_path, &toml_content).with_context(|| {
                format!("Failed to write default keymap file: {}.toml", preset_name)
            })?;
        } else if let Ok(existing) = fs::read_to_string(&preset_path)
            && has_outdated_layout(&existing)
            && let Err(e) = fs::write(&preset_path, &toml_content)
        {
            log::warn!("Failed to refresh preset keymap {:?}: {}", preset_path, e);
        }
    }
    Ok(())
}

/// A preset written before rename moved to F7 and the plugin menu left F11.
fn has_outdated_layout(existing: &str) -> bool {
    (!existing.contains("rename = \"F7\"") && !existing.contains("rename_fkey = \"F7\""))
        || existing.contains("mkdir = \"F7\"")
        || existing.contains("mkdir_fkey = \"F7\"")
        || existing.contains("plugin_menu = \"F11\"")
}

/// Reads `themes/<name>.toml`, falling back to the built-in theme of that name.
pub(super) fn load_theme(theme_name: &str) -> Result<Theme> {
    let builtin = || {
        if theme_name == "classic_blue" {
            Theme::classic_blue()
        } else {
            Theme::default()
        }
    };
    let theme_path = paths::get_themes_dir().join(format!("{}.toml", theme_name));
    if !theme_path.exists() {
        return Ok(builtin());
    }
    let content = fs::read_to_string(&theme_path).context("Failed to read theme file")?;
    Ok(toml::from_str(&content).unwrap_or_else(|_| builtin()))
}
