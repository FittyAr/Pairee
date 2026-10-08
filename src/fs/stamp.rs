//! Size + modification time of a filesystem entry, used to tell whether an
//! entry is still the one an operation left behind.

use std::path::Path;
use std::time::SystemTime;

/// Snapshot of an entry (links are not followed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Stamp {
    /// Stamp of the entry at `path`, `None` when it does not exist.
    pub fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::symlink_metadata(path).ok()?;
        Some(Self {
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }

    /// `true` when the entry at `path` still has this size and time.
    pub fn matches(&self, path: &Path) -> bool {
        Self::of(path).is_some_and(|now| now == *self)
    }
}

/// `true` when an entry of any kind (even a broken link) exists at `path`.
pub fn entry_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_detects_changes_and_missing_entries() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        assert_eq!(Stamp::of(&file), None);
        assert!(!entry_exists(&file));
        std::fs::write(&file, b"one").unwrap();
        let stamp = Stamp::of(&file).unwrap();
        assert_eq!(stamp.size, 3);
        assert!(stamp.matches(&file));
        std::fs::write(&file, b"longer").unwrap();
        assert!(!stamp.matches(&file));
        std::fs::remove_file(&file).unwrap();
        assert!(!stamp.matches(&file));
    }
}
