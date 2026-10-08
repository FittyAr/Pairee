//! What adapters report for one entry, and the panel listing built from it.

use crate::fs::FileEntry;
use crate::fs::list::ListOptions;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One entry of a [`super::Vfs`] listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsEntry {
    pub name: String,
    pub path: PathBuf,
    /// A folder (for links: what the adapter reports for the link target,
    /// when it follows links in listings).
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Size in bytes (meaningless for folders; walkers ignore it).
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// Dot-file, or hidden by a platform attribute.
    pub hidden: bool,
}

impl VfsEntry {
    /// A folder that is not reached through a link: walkers descend into it.
    pub fn is_real_dir(&self) -> bool {
        self.is_dir && !self.is_symlink
    }

    /// A folder entry at `path` named `name` (archive folders, roots).
    pub fn dir(name: String, path: PathBuf) -> Self {
        Self {
            hidden: name.starts_with('.'),
            name,
            path,
            is_dir: true,
            is_symlink: false,
            size: 0,
            modified: None,
        }
    }

    pub fn into_file_entry(self) -> FileEntry {
        FileEntry {
            name: self.name,
            path: self.path,
            size: self.size,
            is_dir: self.is_dir,
            is_symlink: self.is_symlink,
            modified: self.modified,
        }
    }
}

/// The `..` entry shown at the top of the listing of `dir`, if any.
///
/// Folders with a parent get one pointing at it; a root only gets one
/// (pointing at itself) when `show_dotdot_in_root_folders` is set.
pub fn parent_entry(dir: &Path, show_dotdot_in_root_folders: bool) -> Option<FileEntry> {
    let target = match dir.parent() {
        Some(parent) => parent,
        None if show_dotdot_in_root_folders => dir,
        None => return None,
    };
    Some(FileEntry {
        name: "..".to_string(),
        path: target.to_path_buf(),
        size: 0,
        is_dir: true,
        is_symlink: false,
        modified: None,
    })
}

/// `..` plus the visible `children` of `dir`, sorted as `opts` say.
pub fn panel_listing(dir: &Path, children: Vec<VfsEntry>, opts: &ListOptions) -> Vec<FileEntry> {
    let mut entries: Vec<FileEntry> = parent_entry(dir, opts.show_dotdot).into_iter().collect();
    entries.extend(
        children
            .into_iter()
            .filter(|e| opts.show_hidden || !e.hidden)
            .map(VfsEntry::into_file_entry),
    );
    opts.sort(&mut entries);
    entries
}
