use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PluginsLock {
    pub plugins: HashMap<String, PinnedPlugin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinnedPlugin {
    pub version: String,
    pub pinned: bool,
    pub files: HashMap<String, String>, // relative_path -> sha256
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryIndex {
    pub plugins: HashMap<String, RegistryPlugin>,
}

impl RegistryIndex {
    /// The registry version of `name` when it differs from `installed`.
    ///
    /// The registry only lists the latest release, so any difference
    /// (including a lower version) is offered as an update.
    pub fn update_for(&self, name: &str, installed: &str) -> Option<&str> {
        self.plugins
            .get(name)
            .map(|p| p.version.as_str())
            .filter(|latest| *latest != installed)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryPlugin {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub languages: Option<Vec<String>>,
    pub hooks: Option<Vec<String>>,
    pub min_pairee: Option<String>,
}

/// The registry copy of a plugin `manifest.toml`; only its `[files]`
/// table (relative path → SHA-256) matters to the installer.
#[derive(Debug, Deserialize)]
pub struct RegistryPluginManifestWrapper {
    pub files: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Blocklist {
    pub blocked: HashMap<String, String>,
}
