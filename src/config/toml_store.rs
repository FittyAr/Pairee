//! Small TOML state files kept next to the configuration (bookmarks,
//! frecency, history, session): one place that reads them tolerantly and
//! writes them atomically (`0600` on Unix, see [`super::write_atomic`]).
//!
//! A state type implements [`StateFile`] (its file name and folder) and
//! gets `load` / `read` / `save` / `merge_save` from it. Writes take the
//! cross-process lock of [`super::shared_file`].

use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

/// The folder a state file lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// Next to `config.toml` (what the user set up).
    Config,
    /// The cache folder (what Pairee recorded on its own).
    Cache,
}

/// A TOML state file with a fixed name: a missing file reads as nothing,
/// an invalid one is kept aside in `<name>.bak` and reads as nothing, and
/// saves replace the file atomically. The `*_from` / `save_to` variants
/// take an explicit path (tests).
pub trait StateFile: Serialize + DeserializeOwned {
    /// File name inside [`Self::HOME`].
    const FILE_NAME: &'static str;
    const HOME: Home = Home::Config;

    /// Where the file lives.
    fn path() -> PathBuf {
        let dir = match Self::HOME {
            Home::Config => super::paths::get_config_dir(),
            Home::Cache => super::paths::get_cache_dir(),
        };
        dir.join(Self::FILE_NAME)
    }

    /// The saved value, if there is a readable one.
    fn read() -> Option<Self> {
        Self::read_from(&Self::path())
    }

    fn read_from(path: &Path) -> Option<Self> {
        load(path)
    }

    /// The saved value, or the default when missing or unreadable.
    fn load() -> Self
    where
        Self: Default,
    {
        Self::load_from(&Self::path())
    }

    fn load_from(path: &Path) -> Self
    where
        Self: Default,
    {
        Self::read_from(path).unwrap_or_default()
    }

    /// Replaces the file with this value (whole: the last writer wins),
    /// under the lock other instances take ([`super::shared_file`]).
    fn save(&self) -> Result<()> {
        self.save_to(&Self::path())
    }

    fn save_to(&self, path: &Path) -> Result<()> {
        super::shared_file::with_lock(path, || save(path, self))
    }

    /// Under the shared lock: `merge` combines the value on disk (`None`
    /// when missing or unreadable) with this instance's data, and the
    /// result is written and returned.
    fn merge_save(merge: impl FnOnce(Option<Self>) -> Self) -> Result<Self> {
        Self::merge_save_to(&Self::path(), merge)
    }

    fn merge_save_to(path: &Path, merge: impl FnOnce(Option<Self>) -> Self) -> Result<Self> {
        super::shared_file::merge_save(path, merge)
    }
}

/// Result of reading a state file.
#[derive(Debug)]
pub enum Loaded<T> {
    /// The file does not exist yet.
    Missing,
    Ok(T),
    /// The file exists but could not be read or parsed (the message says why).
    Invalid(String),
}

/// Reads and parses `path`.
pub fn read<T: DeserializeOwned>(path: &Path) -> Loaded<T> {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(value) => Loaded::Ok(value),
            Err(e) => Loaded::Invalid(e.to_string()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Loaded::Missing,
        Err(e) => Loaded::Invalid(e.to_string()),
    }
}

/// Reads `path`; `None` when it is missing or invalid. An invalid file is
/// copied to `<name>.bak` first, since the next save replaces it.
pub fn load<T: DeserializeOwned>(path: &Path) -> Option<T> {
    match read(path) {
        Loaded::Ok(value) => Some(value),
        Loaded::Missing => None,
        Loaded::Invalid(e) => {
            set_aside(path, &e);
            None
        }
    }
}

/// Keeps a copy of the invalid file `path` (rejected for `error`) in
/// `<name>.bak`, so writing defaults over it loses nothing.
pub fn set_aside(path: &Path, error: &str) {
    let bak = super::load_guard::backup_path(path);
    match std::fs::copy(path, &bak) {
        Ok(_) => log::warn!(
            "Ignoring invalid {} ({error}); kept a copy in {}",
            path.display(),
            bak.display()
        ),
        Err(copy) => log::warn!(
            "Ignoring invalid {} ({error}); backup failed: {copy}",
            path.display()
        ),
    }
}

/// Serializes `value` and atomically replaces `path` with it.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let body =
        toml::to_string_pretty(value).with_context(|| format!("Serializing {}", path.display()))?;
    super::write_atomic(path, body.as_bytes())
        .with_context(|| format!("Writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        name: String,
        count: u32,
    }

    impl StateFile for Sample {
        const FILE_NAME: &'static str = "sample.toml";
        const HOME: Home = Home::Cache;
    }

    #[test]
    fn state_files_live_in_their_home() {
        let path = Sample::path();
        assert_eq!(path.file_name().unwrap(), "sample.toml");
        assert!(path.starts_with(crate::config::paths::get_cache_dir()));
    }

    #[test]
    fn round_trip_and_missing_or_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sample.toml");
        assert!(matches!(read::<Sample>(&path), Loaded::Missing));

        let value = Sample {
            name: "a".into(),
            count: 3,
        };
        value.save_to(&path).unwrap();
        assert_eq!(Sample::load_from(&path), value);

        std::fs::write(&path, "count = [broken").unwrap();
        assert!(matches!(read::<Sample>(&path), Loaded::Invalid(_)));
        assert_eq!(Sample::load_from(&path), Sample::default());
        assert_eq!(Sample::read_from(&path), None);
        // The next save replaces it, so the invalid file was kept aside.
        assert_eq!(
            std::fs::read_to_string(crate::config::load_guard::backup_path(&path)).unwrap(),
            "count = [broken"
        );
    }
}
