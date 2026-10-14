use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// Creation time, where the file system records it (listing).
    pub created: Option<SystemTime>,
    /// Last access time, where the file system records it (listing).
    pub accessed: Option<SystemTime>,
    /// Owner name: filled in only when the listing sorts by owner, on
    /// platforms that tell it cheaply (Unix).
    pub owner: Option<String>,
    /// `descript.ion` description: filled in only when the listing sorts
    /// by description.
    pub description: Option<String>,
}
