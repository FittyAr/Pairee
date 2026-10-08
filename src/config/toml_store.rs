//! Small TOML state files kept next to the configuration (bookmarks,
//! history, session): one place that reads them tolerantly and writes them
//! atomically (`0600` on Unix, see [`super::write_atomic`]).

use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

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

/// Reads `path`, falling back to the default when it is missing or invalid
/// (an invalid file is logged and left untouched).
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    match read(path) {
        Loaded::Ok(value) => value,
        Loaded::Missing => T::default(),
        Loaded::Invalid(e) => {
            log::warn!("Ignoring invalid {}: {e}", path.display());
            T::default()
        }
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

    #[test]
    fn round_trip_and_missing_or_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sample.toml");
        assert!(matches!(read::<Sample>(&path), Loaded::Missing));

        let value = Sample {
            name: "a".into(),
            count: 3,
        };
        save(&path, &value).unwrap();
        assert_eq!(load_or_default::<Sample>(&path), value);

        std::fs::write(&path, "count = [broken").unwrap();
        assert!(matches!(read::<Sample>(&path), Loaded::Invalid(_)));
        assert_eq!(load_or_default::<Sample>(&path), Sample::default());
    }
}
