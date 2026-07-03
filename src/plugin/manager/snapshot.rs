//! Snapshot of an `AppState` that is sent to plugins through
//! `pairee.sync` / `PluginRequest::GetStateSnapshot`.
//!
//! The shapes here are deliberately serializable so the same struct can
//! cross the mpsc channel (serde_json under the hood) and so that future
//! plugin-side persistence (e.g. caching the state for offline
//! inspection) remains a one-liner.

use crate::fs::FileEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub active_panel: String,
    pub left_cwd: String,
    pub right_cwd: String,
    pub hovered_file: Option<FileEntrySnapshot>,
    pub selected_files: Vec<FileEntrySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntrySnapshot {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl FileEntrySnapshot {
    pub fn from_file_entry(entry: &FileEntry) -> Self {
        let path_str = entry.path.to_string_lossy().to_string();
        Self {
            name: entry.name.clone(),
            url: path_str.clone(),
            path: path_str,
            size: entry.size,
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_entry_snapshot_from_file_entry() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: PathBuf::from("/tmp/test.txt"),
            size: 42,
            is_dir: false,
            is_symlink: false,
            modified: None,
        };
        let snap = FileEntrySnapshot::from_file_entry(&entry);
        assert_eq!(snap.name, "test.txt");
        assert_eq!(snap.url, "/tmp/test.txt");
        assert_eq!(snap.size, 42);
        assert!(!snap.is_dir);
        assert!(!snap.is_symlink);
    }
}
