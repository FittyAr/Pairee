#[cfg(not(target_os = "windows"))]
use directories::ProjectDirs;
use std::path::PathBuf;

/// Returns the platform-specific configuration directory for Pairee.
/// Linux: ~/.config/pairee
/// Windows: %APPDATA%\pairee\config
pub fn get_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(root) = test_root::current() {
        return root.join("config");
    }
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
    #[cfg(test)]
    if let Some(root) = test_root::current() {
        return root.join("cache");
    }
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

/// Returns the private data directory for a single plugin. This is the only
/// location an untrusted plugin may write to through `pairee.fs`.
/// Linux: ~/.local/share/pairee/plugin-data/<name>
/// Windows: %APPDATA%\pairee\plugin-data\<name>
pub fn get_plugin_data_dir(plugin_name: &str) -> PathBuf {
    #[cfg(test)]
    if let Some(root) = test_root::current() {
        return root.join("plugin-data").join(plugin_name);
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var("APPDATA")
        .map(|appdata| PathBuf::from(appdata).join("pairee"))
        .unwrap_or_else(|_| PathBuf::from("."));
    #[cfg(not(target_os = "windows"))]
    let base = ProjectDirs::from("com", "pairee", "Pairee")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("plugin-data").join(plugin_name)
}

/// Directory of an installed plugin: `<config>/plugins/<name>.pairee`.
pub fn get_installed_plugin_dir(plugin_name: &str) -> PathBuf {
    get_config_dir()
        .join("plugins")
        .join(format!("{plugin_name}.pairee"))
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

/// Returns the system-wide sharing directory for Unix installations (e.g. `/usr/share/pairee`).
pub fn get_system_share_dir() -> Option<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let path = PathBuf::from("/usr/share/pairee");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Unit tests never read or write the user's real configuration: every
/// per-user directory lives under a temporary root. A test (the TUI test
/// harness) may point its own thread at a private sandbox with
/// [`test_root::redirect`]; every other thread shares one scratch root.
#[cfg(test)]
pub mod test_root {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    /// The root this thread's per-user directories live under (always set
    /// in tests; an `Option` so the callers fall through in other builds).
    pub fn current() -> Option<PathBuf> {
        let redirected = ROOT.with(|root| root.borrow().clone());
        Some(redirected.unwrap_or_else(shared_root))
    }

    /// One scratch root per test process, so files a previous run (or an
    /// older build) left behind never change what a test sees.
    fn shared_root() -> PathBuf {
        std::env::temp_dir()
            .join("pairee-unit-tests")
            .join(std::process::id().to_string())
    }

    /// Restores the previous root of this thread when dropped.
    pub struct Redirect(Option<PathBuf>);

    /// Points this thread's per-user directories at `root`.
    pub fn redirect(root: &Path) -> Redirect {
        Redirect(ROOT.with(|cell| cell.replace(Some(root.to_path_buf()))))
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            let previous = self.0.take();
            ROOT.with(|cell| *cell.borrow_mut() = previous);
        }
    }
}
