#[cfg(not(target_os = "windows"))]
use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// The per-user folders Pairee keeps its files in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserDir {
    Config,
    Cache,
    /// Plugin data (on Windows and macOS the `FittyAr/Pairee` folder itself).
    Data,
}

impl UserDir {
    /// Sub-folder under the Windows `%APPDATA%\FittyAr\Pairee` folder and the test
    /// root (`None`: the folder itself).
    #[cfg_attr(not(any(test, target_os = "windows")), allow(dead_code))]
    fn subdir(self) -> Option<&'static str> {
        match self {
            Self::Config => Some("config"),
            Self::Cache => Some("cache"),
            Self::Data => None,
        }
    }

    /// Where the folder is (`.`-relative when the platform gives none).
    fn path(self) -> PathBuf {
        #[cfg(test)]
        if let Some(root) = test_root::current() {
            return self.under(root);
        }
        #[cfg(target_os = "windows")]
        {
            let base = std::env::var("APPDATA")
                .map(|appdata| windows_base(Path::new(&appdata)))
                .unwrap_or_else(|_| PathBuf::from("."));
            self.under(base)
        }
        #[cfg(not(target_os = "windows"))]
        {
            match BaseDirs::new() {
                Some(dirs) => {
                    let (new, previous) = self.unix_candidates(&dirs);
                    adopt_previous(&new, &previous)
                }
                None => PathBuf::from("."),
            }
        }
    }

    /// `(folder, folder of the previous layout)` on Linux and macOS.
    ///
    /// Linux (XDG): `$XDG_CONFIG_HOME/FittyAr/Pairee`,
    /// `$XDG_CACHE_HOME/FittyAr/Pairee` and `$XDG_DATA_HOME/FittyAr/Pairee`
    /// (`~/.config`, `~/.cache` and `~/.local/share` when unset), replacing
    /// `<same base>/pairee`.
    ///
    /// macOS: `~/Library/Application Support/FittyAr/Pairee` (settings and
    /// plugin data in one folder, as before) and `~/Library/Caches/FittyAr/Pairee`,
    /// replacing the `com.pairee.Pairee` folders.
    #[cfg(not(target_os = "windows"))]
    fn unix_candidates(self, dirs: &BaseDirs) -> (PathBuf, PathBuf) {
        let base = match self {
            Self::Config => dirs.config_dir(),
            Self::Cache => dirs.cache_dir(),
            Self::Data => dirs.data_dir(),
        };
        let previous = if cfg!(target_os = "macos") {
            "com.pairee.Pairee"
        } else {
            "pairee"
        };
        (fittyar_dir(base), base.join(previous))
    }

    #[cfg_attr(not(any(test, target_os = "windows")), allow(dead_code))]
    fn under(self, base: PathBuf) -> PathBuf {
        match self.subdir() {
            Some(sub) => base.join(sub),
            None => base,
        }
    }
}

/// `<base>/FittyAr/Pairee`: every FittyAr app keeps its files in a shared
/// `FittyAr` folder.
fn fittyar_dir(base: &Path) -> PathBuf {
    base.join("FittyAr").join("Pairee")
}

/// The folder to use: `new`, after renaming `previous` (the folder of the
/// layout before the shared FittyAr one) into place when `new` does not exist
/// yet. Nothing is deleted or merged; if `previous` cannot be moved (another
/// file system, permissions) it keeps being used.
fn adopt_previous(new: &Path, previous: &Path) -> PathBuf {
    if new.exists() || !previous.is_dir() {
        return new.to_path_buf();
    }
    let moved = new
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::rename(previous, new));
    match moved {
        Ok(()) => {
            log::info!("Moved {} to {}", previous.display(), new.display());
            new.to_path_buf()
        }
        Err(e) => {
            log::warn!(
                "Could not move {} to {} ({e}); it keeps being used",
                previous.display(),
                new.display()
            );
            previous.to_path_buf()
        }
    }
}

/// Windows per-user root: `%APPDATA%\FittyAr\Pairee`, replacing `%APPDATA%\pairee`.
#[cfg(target_os = "windows")]
fn windows_base(appdata: &Path) -> PathBuf {
    adopt_previous(&fittyar_dir(appdata), &appdata.join("pairee"))
}

/// Returns the platform-specific configuration directory for Pairee.
/// Linux: ~/.config/FittyAr/Pairee
/// macOS: ~/Library/Application Support/FittyAr/Pairee
/// Windows: %APPDATA%\FittyAr\Pairee\config
pub fn get_config_dir() -> PathBuf {
    UserDir::Config.path()
}

/// Returns the platform-specific cache directory for Pairee (used for logs).
/// Linux: ~/.cache/FittyAr/Pairee
/// macOS: ~/Library/Caches/FittyAr/Pairee
/// Windows: %APPDATA%\FittyAr\Pairee\cache
pub fn get_cache_dir() -> PathBuf {
    UserDir::Cache.path()
}

/// Returns the private data directory for a single plugin. This is the only
/// location an untrusted plugin may write to through `pairee.fs`.
/// Linux: ~/.local/share/FittyAr/Pairee/plugin-data/<name>
/// macOS: ~/Library/Application Support/FittyAr/Pairee/plugin-data/<name>
/// Windows: %APPDATA%\FittyAr\Pairee\plugin-data\<name>
pub fn get_plugin_data_dir(plugin_name: &str) -> PathBuf {
    UserDir::Data.path().join("plugin-data").join(plugin_name)
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

/// Returns the path to the folder of the user's syntax highlighting
/// definitions (`<language>.toml`, added to or replacing the built-in ones).
pub fn get_syntax_dir() -> PathBuf {
    get_config_dir().join("syntax")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_dirs_share_one_root() {
        let root = test_root::current().unwrap();
        assert_eq!(get_config_dir(), root.join("config"));
        assert_eq!(get_cache_dir(), root.join("cache"));
        assert_eq!(get_plugin_data_dir("p"), root.join("plugin-data").join("p"));
        assert_eq!(get_log_file_path(), root.join("cache").join("app.log"));
    }

    fn scratch() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("pairee-paths-")
            .tempdir()
            .unwrap()
    }

    #[test]
    fn the_previous_folder_is_moved_into_place() {
        let tmp = scratch();
        let old = tmp.path().join("pairee");
        std::fs::create_dir_all(old.join("themes")).unwrap();
        std::fs::write(old.join("config.toml"), "x").unwrap();
        let new = fittyar_dir(tmp.path());

        assert_eq!(adopt_previous(&new, &old), new);
        assert_eq!(
            std::fs::read_to_string(new.join("config.toml")).unwrap(),
            "x"
        );
        assert!(new.join("themes").is_dir());
        assert!(!old.exists());
        // A second call keeps the new folder.
        assert_eq!(adopt_previous(&new, &old), new);
    }

    #[test]
    fn an_existing_folder_wins_and_the_previous_one_is_left_alone() {
        let tmp = scratch();
        let old = tmp.path().join("pairee");
        let new = fittyar_dir(tmp.path());
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("config.toml"), "old").unwrap();
        std::fs::create_dir_all(&new).unwrap();

        assert_eq!(adopt_previous(&new, &old), new);
        // Never merged nor deleted.
        assert_eq!(
            std::fs::read_to_string(old.join("config.toml")).unwrap(),
            "old"
        );
        assert!(!new.join("config.toml").exists());
    }

    #[test]
    fn nothing_to_move_gives_the_folder_without_creating_it() {
        let tmp = scratch();
        let new = fittyar_dir(tmp.path());
        assert_eq!(adopt_previous(&new, &tmp.path().join("pairee")), new);
        assert!(!new.exists());
        assert!(!tmp.path().join("FittyAr").exists());
    }

    #[test]
    fn a_previous_file_is_not_moved() {
        let tmp = scratch();
        let old = tmp.path().join("pairee");
        std::fs::write(&old, "not a folder").unwrap();
        let new = fittyar_dir(tmp.path());
        assert_eq!(adopt_previous(&new, &old), new);
        assert!(old.is_file());
    }

    #[test]
    fn a_folder_that_cannot_be_moved_keeps_being_used() {
        let tmp = scratch();
        let old = tmp.path().join("pairee");
        std::fs::create_dir_all(&old).unwrap();
        // `FittyAr` is a file, so `FittyAr/Pairee` cannot be created.
        std::fs::write(tmp.path().join("FittyAr"), "blocker").unwrap();
        let new = fittyar_dir(tmp.path());
        assert_eq!(adopt_previous(&new, &old), old);
        assert!(old.is_dir());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_base_moves_the_old_folder() {
        let tmp = scratch();
        let appdata = tmp.path();
        std::fs::create_dir_all(appdata.join("pairee").join("config")).unwrap();
        std::fs::write(
            appdata.join("pairee").join("config").join("config.toml"),
            "x",
        )
        .unwrap();
        let base = windows_base(appdata);
        assert_eq!(base, appdata.join("FittyAr").join("Pairee"));
        assert!(base.join("config").join("config.toml").is_file());
        assert!(!appdata.join("pairee").exists());
        // A second call keeps the new folder.
        assert_eq!(windows_base(appdata), base);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_folders_live_under_fittyar_and_replace_the_previous_ones() {
        let Some(dirs) = BaseDirs::new() else { return };
        let previous = if cfg!(target_os = "macos") {
            "com.pairee.Pairee"
        } else {
            "pairee"
        };
        for (dir, base) in [
            (UserDir::Config, dirs.config_dir()),
            (UserDir::Cache, dirs.cache_dir()),
            (UserDir::Data, dirs.data_dir()),
        ] {
            let (new, old) = dir.unix_candidates(&dirs);
            assert_eq!(new, base.join("FittyAr").join("Pairee"), "{dir:?}");
            assert_eq!(old, base.join(previous), "{dir:?}");
        }
    }
}
