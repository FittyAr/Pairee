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
    /// Newer registry version, if any.
    pub update_available: Option<String>,
    /// Commands declared in `[permissions] commands` (Secure Mode allowlist).
    pub commands: Vec<String>,
}

/// Build the installed list from the lockfile, the trust settings and an
/// optional registry index (for update badges).
pub fn installed_rows(
    lock: &PluginsLock,
    plugins: &HashMap<String, PluginConfig>,
    index: Option<&RegistryIndex>,
) -> Vec<InstalledPlugin> {
    let mut installed: Vec<InstalledPlugin> = lock
        .plugins
        .iter()
        .map(|(name, info)| InstalledPlugin {
            name: name.clone(),
            version: info.version.clone(),
            pinned: info.pinned,
            trusted: plugins.get(name).is_some_and(|p| p.trusted),
            update_available: index
                .and_then(|idx| idx.plugins.get(name))
                .filter(|reg| reg.version != info.version)
                .map(|reg| reg.version.clone()),
            commands: declared_commands(name),
        })
        .collect();
    installed.sort_by(|a, b| a.name.cmp(&b.name));
    installed
}

fn declared_commands(name: &str) -> Vec<String> {
    let dir = crate::config::paths::get_installed_plugin_dir(name);
    crate::plugin::loader::PluginManifest::read_from_dir(&dir)
        .map(|m| m.permissions.commands)
        .unwrap_or_default()
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
        let rows = installed_rows(&lock, &plugins, Some(&index));
        assert_eq!(rows.len(), 1);
        assert!(rows[0].trusted && rows[0].pinned);
        assert_eq!(rows[0].update_available.as_deref(), Some("1.1.0"));
        assert!(rows[0].commands.is_empty());
    }
}
