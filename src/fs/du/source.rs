//! Directory listing backends for the size walker.

use std::io;
use std::path::{Path, PathBuf};

/// What a directory entry is, as far as the walker is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuKind {
    /// A real directory (not a link to one): descended into.
    Dir,
    /// A file, symbolic link, junction or special file: counted, not followed.
    File,
    /// Its metadata could not be read: counted as nothing, marks the scan partial.
    Unreadable,
}

/// One child of a listed directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: DuKind,
    /// Apparent size in bytes (0 for directories).
    pub size: u64,
    /// `(device, inode)` of a file with more than one hard link, so it is
    /// counted once; `None` otherwise.
    pub id: Option<(u64, u64)>,
}

/// Lists one directory without following symbolic links.
pub trait DuSource: Sync {
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<DuEntry>>;
}

/// The local filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalSource;

impl DuSource for LocalSource {
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<DuEntry>> {
        Ok(std::fs::read_dir(dir)?
            .map(|item| match item {
                Ok(entry) => local_entry(&entry),
                Err(_) => unreadable(dir.to_path_buf(), String::new()),
            })
            .collect())
    }
}

fn unreadable(path: PathBuf, name: String) -> DuEntry {
    DuEntry {
        name,
        path,
        kind: DuKind::Unreadable,
        size: 0,
        id: None,
    }
}

fn local_entry(entry: &std::fs::DirEntry) -> DuEntry {
    let name = entry.file_name().to_string_lossy().into_owned();
    let path = entry.path();
    // `DirEntry::metadata` does not traverse symbolic links.
    let Ok(meta) = entry.metadata() else {
        return unreadable(path, name);
    };
    let is_dir = meta.file_type().is_dir();
    DuEntry {
        name,
        path,
        kind: if is_dir { DuKind::Dir } else { DuKind::File },
        size: if is_dir { 0 } else { meta.len() },
        id: if is_dir { None } else { hard_link_id(&meta) },
    }
}

#[cfg(unix)]
fn hard_link_id(meta: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    (meta.nlink() > 1).then(|| (meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn hard_link_id(_meta: &std::fs::Metadata) -> Option<(u64, u64)> {
    None
}
