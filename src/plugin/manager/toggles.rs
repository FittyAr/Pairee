//! Plugin Manager switches that change how plugins run: a plugin's trust,
//! whether it is enabled, and the global Secure Mode. Each change is saved
//! to `config.toml` (through the host's config, so a locked invalid file is
//! not overwritten) and applied at once by reloading the plugins.

use super::lifecycle::PluginManager;
use crate::config::AppConfig;
use crate::config::settings::PluginConfig;

/// What a Plugin Manager switch changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginToggle {
    Trust,
    Enabled,
}

/// Flips `toggle` for plugin `name` in `config`; returns the new value.
pub fn flip(config: &mut AppConfig, name: &str, toggle: PluginToggle) -> bool {
    let entry = config
        .settings
        .plugins
        .entry(name.to_string())
        .or_insert_with(|| PluginConfig::new(name));
    let flag = match toggle {
        PluginToggle::Trust => &mut entry.trusted,
        PluginToggle::Enabled => &mut entry.enabled,
    };
    *flag = !*flag;
    let value = *flag;
    apply(config);
    value
}

/// Flips the global Secure Mode; returns the new value.
pub fn flip_secure_mode(config: &mut AppConfig) -> bool {
    config.settings.secure_mode = !config.settings.secure_mode;
    apply(config);
    config.settings.secure_mode
}

/// Replaces the user values of plugin `name`'s settings (only those that
/// differ from the schema defaults; none removes the table).
pub fn set_settings(
    config: &mut AppConfig,
    name: &str,
    values: std::collections::HashMap<String, String>,
) {
    let all = &mut config.settings.plugin_settings;
    if values.is_empty() {
        all.remove(name);
    } else {
        all.insert(name.to_string(), values);
    }
    apply(config);
}

/// Drops everything `config` keeps about plugin `name` (trust, enabled
/// flag, settings) when it is uninstalled; whether anything was there.
pub fn forget(config: &mut AppConfig, name: &str) -> bool {
    let trust = config.settings.plugins.remove(name).is_some();
    let values = config.settings.plugin_settings.remove(name).is_some();
    trust || values
}

/// [`forget`] on the saved `config.toml`, for callers that hold no config
/// (the CLI, the updater removing a blocked plugin).
pub fn forget_saved(name: &str) -> anyhow::Result<()> {
    let mut config = AppConfig::load_or_create()?;
    if forget(&mut config, name) {
        config.save()?;
    }
    Ok(())
}

/// Saves `config` and reloads the plugins with it (also used by the
/// configuration dialog when its Secure Mode row changed).
pub fn apply(config: &AppConfig) {
    config.save_logging();
    PluginManager::reload_all(config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_creates_the_entry_and_toggles() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let mut config = AppConfig::default();
        assert!(flip(&mut config, "demo", PluginToggle::Trust));
        assert!(config.settings.plugins["demo"].enabled);
        assert!(!flip(&mut config, "demo", PluginToggle::Enabled));
        assert!(config.settings.plugins["demo"].trusted);
        assert!(flip_secure_mode(&mut config));
    }

    #[test]
    fn uninstalling_forgets_trust_and_settings_on_disk_too() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let mut config = AppConfig::default();
        flip(&mut config, "demo", PluginToggle::Trust);
        flip(&mut config, "other", PluginToggle::Trust);
        set_settings(
            &mut config,
            "demo",
            [("a".to_string(), "1".to_string())].into(),
        );
        assert!(forget(&mut config, "demo"));
        assert!(!forget(&mut config, "demo"));
        assert!(config.settings.plugins.contains_key("other"));
        // A fresh install of `demo` must not come back trusted.
        let mut again = AppConfig::default();
        flip(&mut again, "demo", PluginToggle::Trust);
        forget_saved("demo").unwrap();
        let reloaded = AppConfig::load_or_create().unwrap();
        assert!(!reloaded.settings.plugins.contains_key("demo"));
    }

    #[test]
    fn settings_are_replaced_and_empty_ones_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let mut config = AppConfig::default();
        let values = [("depth".to_string(), "4".to_string())].into();
        set_settings(&mut config, "demo", values);
        assert_eq!(config.settings.plugin_settings["demo"]["depth"], "4");
        set_settings(&mut config, "demo", Default::default());
        assert!(!config.settings.plugin_settings.contains_key("demo"));
    }
}
