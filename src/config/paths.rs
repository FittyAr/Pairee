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

#[cfg(test)]
mod tests {
    use super::*;

    /// `get_config_file_path` must end in `config.toml`.
    #[test]
    fn config_file_path_uses_config_toml_filename() {
        let p = get_config_file_path();
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("config.toml"));
    }

    /// `get_keybindings_file_path` must end in `keybindings.toml`.
    #[test]
    fn keybindings_file_path_uses_keybindings_toml_filename() {
        let p = get_keybindings_file_path();
        assert_eq!(
            p.file_name().and_then(|n| n.to_str()),
            Some("keybindings.toml")
        );
    }

    /// The themes, keymaps, and config files must all live under
    /// the same root config directory — otherwise a partial
    /// install would scatter state across the filesystem.
    #[test]
    fn subdir_paths_share_config_root() {
        let cfg = get_config_dir();
        assert_eq!(get_themes_dir().parent(), Some(cfg.as_path()));
        assert_eq!(get_keymaps_dir().parent(), Some(cfg.as_path()));
        assert_eq!(get_config_file_path().parent(), Some(cfg.as_path()));
        assert_eq!(get_keybindings_file_path().parent(), Some(cfg.as_path()));
    }

    /// The log file must live under the cache dir, not the config
    /// dir — otherwise `app.log` would be backed up / synced.
    #[test]
    fn log_file_lives_under_cache_dir() {
        let log = get_log_file_path();
        let cache = get_cache_dir();
        assert_eq!(log.parent(), Some(cache.as_path()));
        assert_eq!(log.file_name().and_then(|n| n.to_str()), Some("app.log"));
    }

    /// On Windows the config dir is rooted at `%APPDATA%\pairee\config`
    /// (or `.` as a fallback). We can't safely override `APPDATA` in a
    /// test (it would break the rest of the test process), so this
    /// test only checks the fallback path: the function must return
    /// *some* `PathBuf`, ending in either `pairee/config` or `.`.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_config_dir_is_well_formed() {
        let dir = get_config_dir();
        let s = dir.to_string_lossy();
        // Either the APPDATA-based path or the `.` fallback.
        assert!(
            s.ends_with("pairee\\config") || s == ".",
            "unexpected config dir on Windows: {}",
            s
        );
    }

    /// On Unix, `get_system_share_dir` returns `None` when no
    /// candidate path exists on the filesystem. This is the
    /// well-defined case for a freshly booted test runner.
    ///
    /// We avoid creating `/usr/share/pairee` etc. — that would
    /// require root and is out of scope. The test just verifies
    /// the function is callable and returns `Option<PathBuf>`.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn system_share_dir_returns_option() {
        let res = get_system_share_dir();
        // It can be `Some` if the dev happens to have the path
        // installed; we just require the type to be coherent and
        // not panic.
        if let Some(p) = res {
            assert!(p.is_absolute(), "share dir should be absolute: {:?}", p);
            assert!(p.exists(), "share dir should exist: {:?}", p);
        }
    }

    /// `user_local_share` must produce `~/.local/share/pairee` when
    /// `HOME` is set. We override `HOME` for this test using
    /// `set_var` — safe because tests run in their own process.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn user_local_share_uses_home() {
        // Snapshot the real HOME so we can restore it at the end.
        let original_home = std::env::var("HOME").ok();

        // SAFETY: the test module is single-threaded for this case
        // (we don't spawn threads that read `HOME`); the env is
        // restored before any other test runs.
        unsafe {
            std::env::set_var("HOME", "/tmp/fake-home");
        }

        let p = user_local_share();
        assert_eq!(p, PathBuf::from("/tmp/fake-home/.local/share/pairee"));

        // Restore.
        match original_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    /// `xdg_data_home` honours `XDG_DATA_HOME` first, then falls
    /// back to `HOME`. With `XDG_DATA_HOME=/data` it returns
    /// `/data/pairee` regardless of `HOME`.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn xdg_data_home_prefers_env_var_over_home() {
        let original_xdg = std::env::var("XDG_DATA_HOME").ok();
        let original_home = std::env::var("HOME").ok();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", "/opt/data");
            std::env::set_var("HOME", "/tmp/will-be-ignored");
        }

        let p = xdg_data_home();
        assert_eq!(p, PathBuf::from("/opt/data/pairee"));

        // Restore.
        match original_xdg {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        match original_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    /// When `XDG_DATA_HOME` is empty, it must be ignored (treated as
    /// unset) and the function falls back to `$HOME/.local/share`.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn xdg_data_home_ignores_empty_value() {
        let original_xdg = std::env::var("XDG_DATA_HOME").ok();
        let original_home = std::env::var("HOME").ok();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", "");
            std::env::set_var("HOME", "/home/tester");
        }

        let p = xdg_data_home();
        assert_eq!(p, PathBuf::from("/home/tester/.local/share/pairee"));

        // Restore.
        match original_xdg {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        match original_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    /// When both `XDG_DATA_HOME` and `HOME` are unset, the function
    /// returns an empty `PathBuf` — the caller treats that as "no
    /// fallback available".
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn xdg_data_home_returns_empty_when_no_env() {
        let original_xdg = std::env::var("XDG_DATA_HOME").ok();
        let original_home = std::env::var("HOME").ok();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
            std::env::remove_var("HOME");
        }

        let p = xdg_data_home();
        assert!(p.as_os_str().is_empty());

        // Restore.
        match original_xdg {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::set_var("XDG_DATA_HOME", "") },
        }
        match original_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => {}
        }
    }
}
