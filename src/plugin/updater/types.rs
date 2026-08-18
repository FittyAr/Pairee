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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegistryPluginManifestWrapper {
    pub plugin: Option<RegistryPlugin>,
    pub files: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Blocklist {
    pub blocked: HashMap<String, String>,
}
