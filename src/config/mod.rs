pub mod associations;
pub mod history;
pub mod keybindings;
pub mod localization;
pub mod paths;
pub mod settings;
pub mod theme;

use anyhow::{Context, Result};
use keybindings::KeybindingsConfig;
use settings::Settings;
use std::fs;
use theme::Theme;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub settings: Settings,
    pub theme: Theme,
    pub keybindings: KeybindingsConfig,
}

impl AppConfig {
    /// Loads the settings, theme, and keybindings from disk.
    /// If the configuration directory or files do not exist, they are created with default values.
    pub fn load_or_create() -> Result<Self> {
        let config_dir = paths::get_config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;
        }

        let cache_dir = paths::get_cache_dir();
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
        }

        // 1. Settings Loading
        let settings_path = paths::get_config_file_path();
        let settings: Settings = if settings_path.exists() {
            let content =
                fs::read_to_string(&settings_path).context("Failed to read config.toml")?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            let default_settings = Settings::default();
            let toml_str = toml::to_string_pretty(&default_settings)
                .context("Failed to serialize default settings")?;
            fs::write(&settings_path, toml_str).context("Failed to write default config.toml")?;
            default_settings
        };

        // Initialize default translations and load active language
        let _ = localization::init_default_languages();
        localization::load_language(&settings.language);

        // 2. Keybindings Loading
        let keybindings_path = paths::get_keybindings_file_path();
        let keybindings: KeybindingsConfig = if keybindings_path.exists() {
            let content =
                fs::read_to_string(&keybindings_path).context("Failed to read keybindings.toml")?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            let default_keybindings = KeybindingsConfig::default();
            let toml_str = toml::to_string_pretty(&default_keybindings)
                .context("Failed to serialize default keybindings")?;
            fs::write(&keybindings_path, toml_str)
                .context("Failed to write default keybindings.toml")?;
            default_keybindings
        };

        // 3. Theme Loading
        let theme_name = &settings.theme;
        let theme: Theme = if theme_name == "classic_blue" {
            Theme::classic_blue()
        } else if theme_name == "slate" {
            Theme::default()
        } else {
            let themes_dir = paths::get_themes_dir();
            let theme_path = themes_dir.join(format!("{}.toml", theme_name));
            if theme_path.exists() {
                let content =
                    fs::read_to_string(&theme_path).context("Failed to read theme file")?;
                toml::from_str(&content).unwrap_or_default()
            } else {
                // Default fallback
                Theme::default()
            }
        };

        Ok(Self {
            settings,
            theme,
            keybindings,
        })
    }

    /// Persists the active configuration back to the disk.
    pub fn save(&self) -> Result<()> {
        let settings_path = paths::get_config_file_path();
        let settings_toml = toml::to_string_pretty(&self.settings)?;
        fs::write(settings_path, settings_toml)?;

        let keybindings_path = paths::get_keybindings_file_path();
        let keybindings_toml = toml::to_string_pretty(&self.keybindings)?;
        fs::write(keybindings_path, keybindings_toml)?;

        Ok(())
    }
}
