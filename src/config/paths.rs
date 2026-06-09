#[cfg(not(target_os = "windows"))]
use directories::ProjectDirs;
use std::path::PathBuf;

/// Returns the platform-specific configuration directory for NCRust.
/// Linux: ~/.config/ncrust
/// Windows: %APPDATA%\ncrust\config
pub fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("ncrust").join("config")
        } else {
            PathBuf::from(".").join("config")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ProjectDirs::from("com", "ncrust", "NCRust")
            .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Returns the platform-specific cache directory for NCRust (used for logs).
/// Linux: ~/.cache/ncrust
/// Windows: %APPDATA%\ncrust\cache
pub fn get_cache_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("ncrust").join("cache")
        } else {
            PathBuf::from(".").join("cache")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ProjectDirs::from("com", "ncrust", "NCRust")
            .map(|proj_dirs| proj_dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Returns the path to the main config.toml file.
pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

/// Returns the path to the keybindings override file.
pub fn get_keybindings_file_path() -> PathBuf {
    get_config_dir().join("keybindings.toml")
}

/// Returns the path to the themes subdirectory.
pub fn get_themes_dir() -> PathBuf {
    get_config_dir().join("themes")
}

/// Returns the path to the application log file.
pub fn get_log_file_path() -> PathBuf {
    get_cache_dir().join("app.log")
}
