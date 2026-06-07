use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// The name of the file or folder (e.g. "docs")
    pub name: String,
    /// The absolute path to the file or folder
    pub path: PathBuf,
    /// Size of the file in bytes
    pub size: u64,
    /// True if the entry represents a directory
    pub is_dir: bool,
    /// True if the entry is a symbolic link
    pub is_symlink: bool,
    /// The last modification time, if available
    pub modified: Option<SystemTime>,
}
