//! Snapshot of installed plugins shown in the Plugin Manager.

use crate::config::settings::PluginConfig;
use crate::plugin::updater::RegistryIndex;
use crate::plugin::updater::types::PluginsLock;
use std::collections::HashMap;

/// One row of the Plugin Manager "Installed" tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub pinned: bool,
    pub trusted: bool,
    /// Loaded at startup / on reload (the user can disable it).
    pub enabled: bool,
    /// Secure Mode as plugins currently see it (global, repeated per row).
    pub secure_mode: bool,
    /// Newer registry version, if any.
    pub update_available: Option<String>,
    /// Commands declared in `[permissions] commands` (Secure Mode allowlist).
    pub commands: Vec<String>,
    /// Entries of `[settings_schema]` (editable with `c` / Enter).
    pub settings: usize,
}

/// Build the installed list from the lockfile, the trust settings, the
/// effective Secure Mode and an optional registry index (update badges).
pub fn installed_rows(
    lock: &PluginsLock,
    plugins: &HashMap<String, PluginConfig>,
    secure_mode: bool,
    index: Option<&RegistryIndex>,
) -> Vec<InstalledPlugin> {
    let mut installed: Vec<InstalledPlugin> = lock
        .plugins
        .iter()
        .map(|(name, info)| {
            let manifest = crate::plugin::loader::PluginManifest::read_from_dir(
                &crate::config::paths::get_installed_plugin_dir(name),
            );
            InstalledPlugin {
                name: name.clone(),
                version: info.version.clone(),
                pinned: info.pinned,
                trusted: plugins.get(name).is_some_and(|p| p.trusted),
                enabled: plugins.get(name).is_none_or(|p| p.enabled),
                secure_mode,
                update_available: index
                    .and_then(|idx| idx.plugins.get(name))
                    .filter(|reg| reg.version != info.version)
                    .map(|reg| reg.version.clone()),
                commands: manifest
                    .as_ref()
                    .map(|m| m.permissions.commands.clone())
                    .unwrap_or_default(),
                settings: manifest
                    .as_ref()
                    .map_or(0, |m| crate::plugin::runtime::settings::specs(m).len()),
            }
        })
        .collect();
    installed.sort_by(|a, b| a.name.cmp(&b.name));
    installed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::updater::{PinnedPlugin, RegistryPlugin};

    #[test]
    fn builds_rows_with_trust_and_updates() {
        let mut lock = PluginsLock::default();
        lock.plugins.insert(
            "zz-demo-not-installed".into(),
            PinnedPlugin {
                version: "1.0.0".into(),
                pinned: true,
                files: HashMap::new(),
            },
        );
        let mut plugins = HashMap::new();
        plugins.insert(
            "zz-demo-not-installed".to_string(),
            PluginConfig {
                name: "zz-demo-not-installed".into(),
                trusted: true,
                enabled: false,
            },
        );
        let mut index = RegistryIndex {
            plugins: HashMap::new(),
        };
        index.plugins.insert(
            "zz-demo-not-installed".into(),
            RegistryPlugin {
                name: "zz-demo-not-installed".into(),
                version: "1.1.0".into(),
                description: None,
                author: None,
                languages: None,
                hooks: None,
                min_pairee: None,
            },
        );
        let rows = installed_rows(&lock, &plugins, true, Some(&index));
        assert_eq!(rows.len(), 1);
        assert!(rows[0].trusted && rows[0].pinned && rows[0].secure_mode);
        assert!(!rows[0].enabled);
        assert_eq!(rows[0].update_available.as_deref(), Some("1.1.0"));
        assert!(rows[0].commands.is_empty());
        assert_eq!(rows[0].settings, 0);
    }
}
