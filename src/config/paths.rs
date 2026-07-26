#[cfg(not(target_os = "windows"))]
use directories::ProjectDirs;
use std::path::PathBuf;

/// Returns the platform-specific configuration directory for Pairee.
/// Linux: ~/.config/pairee
/// Windows: %APPDATA%\pairee\config
pub fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("pairee").join("config")
        } else {
            PathBuf::from(".").join("config")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ProjectDirs::from("com", "pairee", "Pairee")
            .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Returns the platform-specific cache directory for Pairee (used for logs).
/// Linux: ~/.cache/pairee
/// Windows: %APPDATA%\pairee\cache
pub fn get_cache_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("pairee").join("cache")
        } else {
            PathBuf::from(".").join("cache")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ProjectDirs::from("com", "pairee", "Pairee")
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

/// Returns the path to the keymaps subdirectory where preset TOML files live.
/// Each file is named `<preset_name>.toml` (e.g. `norton.toml`, `neovim.toml`).
pub fn get_keymaps_dir() -> PathBuf {
    get_config_dir().join("keymaps")
}

/// Returns the path to the application log file.
pub fn get_log_file_path() -> PathBuf {
    get_cache_dir().join("app.log")
}

/// Returns the system-wide sharing directory for Unix installations
/// (e.g. `/usr/share/pairee` for distro packages, `~/.local/share/pairee`
/// for user-local installs, and the XDG data dir as a last resort).
///
/// We used to check only `/usr/share/pairee`, which meant user-local
/// installs (the default produced by `install.sh` on a non-sudo box) and
/// distro-packaged installs that live under `/usr/local/share` both
/// silently fell back to "no translations, no help, no themes" even
/// though the assets were right there in the user's `~/.local/share`.
pub fn get_system_share_dir() -> Option<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        // Ordered most-specific to least-specific; first match wins.
        let candidates: [PathBuf; 5] = [
            PathBuf::from("/usr/share/pairee"),
            PathBuf::from("/usr/local/share/pairee"),
            // User-local install (matches `install.sh` behaviour).
            user_local_share(),
            // XDG data dir (covers `~/.local/share` and the
            // `$XDG_DATA_HOME` override).
            xdg_data_home(),
            // Flatpak location (matches the app id when packaged).
            PathBuf::from("/var/lib/flatpak/exports/share/pairee"),
        ];
        for c in candidates.iter() {
            if !c.as_os_str().is_empty() && c.exists() {
                return Some(c.clone());
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn user_local_share() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("pairee");
    }
    PathBuf::new()
}

#[cfg(not(target_os = "windows"))]
fn xdg_data_home() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("pairee");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("pairee");
    }
    PathBuf::new()
}
