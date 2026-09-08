use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshPreset {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PluginConfig {
    pub name: String,
    #[serde(default)]
    pub trusted: bool,
}

pub fn default_true() -> bool {
    true
}

pub fn default_git_log_limit() -> u32 {
    100
}

pub fn default_plugins_dev_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            std::path::PathBuf::from(appdata)
                .join("pairee")
                .join("config")
                .join("plugins")
                .to_string_lossy()
                .into_owned()
        } else {
            "./config/plugins".to_string()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::config::paths::get_config_dir()
            .join("plugins")
            .to_string_lossy()
            .into_owned()
    }
}

pub fn default_transfer_hash() -> String {
    "blake3".to_string()
}
pub fn default_transfer_buffer() -> u32 {
    1024 * 1024
}
pub fn default_transfer_max_retries() -> u32 {
    3
}
pub fn default_transfer_conflict() -> String {
    "ask".to_string()
}
pub fn default_transfer_report_format() -> String {
    "html".to_string()
}
