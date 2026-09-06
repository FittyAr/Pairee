//! Snapshot of an `AppState` that is sent to plugins through
//! `pairee.sync` / `PluginRequest::GetStateSnapshot`.
//!
//! The shapes here are deliberately serializable so the same struct can
//! cross the mpsc channel (serde_json under the hood) and so that future
//! plugin-side persistence (e.g. caching the state for offline
//! inspection) remains a one-liner.

use crate::fs::FileEntry;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub mime: String,
    pub mtime: Option<u64>,
    pub is_hidden: bool,
    pub is_exec: bool,
}

pub(crate) fn guess_mime(name: &str, is_dir: bool) -> String {
    if is_dir {
        return "inode/directory".into();
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "log" | "md" | "rst" | "toml" | "json" | "yml" | "yaml" | "xml" | "csv" | "ini" => {
            "text/plain"
        }
        "rs" | "py" | "js" | "ts" | "go" | "c" | "h" | "cpp" | "lua" | "sh" | "ps1" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" | "ico" | "tif" | "tiff" => "image/png",
        "pdf" => "application/pdf",
        "zip" | "7z" => "application/zip",
        "gz" | "tgz" | "tar" => "application/gzip",
        _ => "application/octet-stream",
    }
    .into()
}

pub(crate) fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.') && name != "." && name != ".."
}

pub(crate) fn mtime_unix(modified: Option<SystemTime>) -> Option<u64> {
    modified
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

pub(crate) fn is_executable(path: &Path, name: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::symlink_metadata(path)
            && meta.permissions().mode() & 0o111 != 0
        {
            return true;
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com" | "ps1")
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
            mime: guess_mime(&entry.name, entry.is_dir),
            mtime: mtime_unix(entry.modified),
            is_hidden: is_hidden_name(&entry.name),
            is_exec: is_executable(&entry.path, &entry.name),
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
        assert_eq!(snap.mime, "text/plain");
        assert!(!snap.is_hidden);
    }

    #[test]
    fn guess_mime_covers_dir_image_and_unknown() {
        assert_eq!(guess_mime("docs", true), "inode/directory");
        assert_eq!(guess_mime("photo.JPEG", false), "image/jpeg");
        assert_eq!(guess_mime("weird.bin", false), "application/octet-stream");
        assert!(is_hidden_name(".gitignore"));
        assert!(!is_hidden_name(".."));
        assert!(!is_hidden_name("readme"));
    }
}
