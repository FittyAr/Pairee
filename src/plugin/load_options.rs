//! What the host decides for each plugin before it runs: whether it loads
//! at all, its trust, Secure Mode and its user settings.
//!
//! The values come from the host's already loaded [`AppConfig`] (never from
//! a fresh `AppConfig::load_or_create()` inside a plugin task), so loading
//! plugins neither re-reads nor rewrites `config.toml`, and every plugin of
//! one load sees the same snapshot.

use crate::config::AppConfig;
use crate::config::settings::Settings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Host decisions for one plugin, captured by its Lua runtime.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginLoadOptions {
    /// Plugin key (folder name without `.pairee`), as used in `config.toml`.
    pub name: String,
    pub trusted: bool,
    pub secure_mode: bool,
    /// `plugin_settings.<name>` from `config.toml` (string values).
    pub settings: HashMap<String, String>,
}

impl PluginLoadOptions {
    /// Options with no user settings (tests).
    #[cfg(test)]
    pub fn new(name: &str, trusted: bool, secure_mode: bool) -> Self {
        Self {
            name: name.to_string(),
            trusted,
            secure_mode,
            settings: HashMap::new(),
        }
    }

    fn from_settings(name: &str, settings: &Settings, secure_mode: bool) -> Self {
        Self {
            name: name.to_string(),
            trusted: settings.plugins.get(name).is_some_and(|c| c.trusted),
            secure_mode,
            settings: settings
                .plugin_settings
                .get(name)
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// Secure Mode as plugins see it: on when the user enabled it, and also
/// when `config.toml` could not be parsed (fail closed: the user's real
/// choice is unknown, so the sandbox stays on).
pub fn effective_secure_mode(config: &AppConfig) -> bool {
    config.settings.secure_mode || config.settings_save_locked()
}

/// `true` unless the user disabled `name` in the Plugin Manager.
pub fn is_enabled(settings: &Settings, name: &str) -> bool {
    settings.plugins.get(name).is_none_or(|c| c.enabled)
}

/// One plugin to load: its folder and the options it runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedPlugin {
    pub dir: PathBuf,
    pub options: PluginLoadOptions,
}

/// The enabled plugins under `plugins_dir` (folders named `<name>.pairee`),
/// with the options `config` gives them. Empty when plugins are off.
pub fn load_plan(config: &AppConfig, plugins_dir: &Path) -> Vec<PlannedPlugin> {
    if !config.settings.plugins_enabled {
        return Vec::new();
    }
    let secure_mode = effective_secure_mode(config);
    let Ok(entries) = std::fs::read_dir(plugins_dir) else {
        return Vec::new();
    };
    let mut plan: Vec<PlannedPlugin> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            let folder = dir.file_name()?.to_str()?.to_string();
            let name = folder.strip_suffix(".pairee")?;
            if !is_enabled(&config.settings, name) {
                tracing::info!("Plugin {name} is disabled; not loading it");
                return None;
            }
            let options = PluginLoadOptions::from_settings(name, &config.settings, secure_mode);
            Some(PlannedPlugin { dir, options })
        })
        .collect();
    plan.sort_by(|a, b| a.options.name.cmp(&b.options.name));
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_guard::ConfigLoadState;
    use crate::config::settings::PluginConfig;

    fn config_with(plugins: &[(&str, bool, bool)]) -> AppConfig {
        let mut config = AppConfig::default();
        for (name, trusted, enabled) in plugins {
            config.settings.plugins.insert(
                name.to_string(),
                PluginConfig {
                    name: name.to_string(),
                    trusted: *trusted,
                    enabled: *enabled,
                },
            );
        }
        config
    }

    #[test]
    fn disabled_plugins_are_not_planned() {
        let dir = tempfile::tempdir().unwrap();
        for folder in ["on.pairee", "off.pairee", "new.pairee", "not-a-plugin"] {
            std::fs::create_dir(dir.path().join(folder)).unwrap();
        }
        let config = config_with(&[("on", true, true), ("off", true, false)]);
        let plan = load_plan(&config, dir.path());
        let names: Vec<&str> = plan.iter().map(|p| p.options.name.as_str()).collect();
        assert_eq!(names, vec!["new", "on"]);
        assert!(plan[1].options.trusted);
        assert!(!plan[0].options.trusted);
    }

    #[test]
    fn plugins_off_plans_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a.pairee")).unwrap();
        let mut config = AppConfig::default();
        config.settings.plugins_enabled = false;
        assert!(load_plan(&config, dir.path()).is_empty());
    }

    #[test]
    fn unparsable_config_forces_secure_mode() {
        let mut config = AppConfig::default();
        assert!(!effective_secure_mode(&config));
        config.load_state = ConfigLoadState::SaveLocked;
        assert!(effective_secure_mode(&config));
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a.pairee")).unwrap();
        assert!(load_plan(&config, dir.path())[0].options.secure_mode);
    }

    #[test]
    fn user_settings_reach_the_options() {
        let mut settings = Settings::default();
        settings.plugin_settings.insert(
            "demo".into(),
            HashMap::from([("depth".to_string(), "3".to_string())]),
        );
        let options = PluginLoadOptions::from_settings("demo", &settings, true);
        assert_eq!(options.settings.get("depth").map(String::as_str), Some("3"));
        assert!(options.secure_mode && !options.trusted);
    }
}
