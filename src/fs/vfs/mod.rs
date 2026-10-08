//! Panel source port (Ports & Adapters).
//!
//! [`Vfs`] is the one interface panels, folder sizes, multi-rename, compare,
//! the viewer and quick view use to read (and, when the adapter can, change)
//! the place a panel shows. Adapters:
//!
//! * [`LocalVfs`] — the local filesystem;
//! * `SharedSshClient` (in [`crate::fs::ssh`]) — an SFTP server;
//! * [`crate::fs::archive::ArchiveVfs`] — the inside of a zip / tar / 7z file.
//!
//! Every adapter reports what it supports through [`Capabilities`]; the
//! operations it does not support return [`unsupported`] errors, so callers
//! check the flags first and show a message instead of failing half-way.

mod entry;
mod local;
mod source;

#[cfg(test)]
pub(crate) mod contract;

pub use entry::{VfsEntry, panel_listing, parent_entry};
pub use local::LocalVfs;
pub use source::PanelSource;

use crate::fs::FileEntry;
use crate::fs::du::{DuEntry, DuKind};
use crate::fs::list::ListOptions;
use crate::fs::text::ByteStore;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What an adapter can do besides listing and reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    /// Create or replace files ([`Vfs::write_file`]).
    pub write: bool,
    /// Create folders ([`Vfs::mkdir`]).
    pub mkdir: bool,
    /// Delete files and folders ([`Vfs::remove_all`]).
    pub remove: bool,
    /// Rename entries in place ([`Vfs::rename`]).
    pub rename: bool,
    /// Paths are real local paths, so tools that run on local files
    /// (editor, attributes, links, wipe, archivers, shell commands) work.
    pub local_tools: bool,
}

impl Capabilities {
    /// Listing and reading only.
    pub const READ_ONLY: Self = Self {
        write: false,
        mkdir: false,
        remove: false,
        rename: false,
        local_tools: false,
    };

    /// Everything (the local filesystem).
    pub const FULL: Self = Self {
        write: true,
        mkdir: true,
        remove: true,
        rename: true,
        local_tools: true,
    };

    pub fn allows(self, capability: Capability) -> bool {
        match capability {
            Capability::Write => self.write,
            Capability::MkDir => self.mkdir,
            Capability::Remove => self.remove,
            Capability::Rename => self.rename,
            Capability::LocalTools => self.local_tools,
        }
    }
}

/// One flag of [`Capabilities`], as an action requires it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Write,
    MkDir,
    Remove,
    Rename,
    LocalTools,
}

/// The error an adapter returns for an operation it does not support.
pub fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        crate::config::localization::t("vfs_unsupported_operation"),
    )
}

/// A browsable filesystem (the port). Paths are absolute in the adapter's
/// own namespace: local paths, remote paths, or `archive.zip/inner/path`.
pub trait Vfs: Send + Sync + std::fmt::Debug {
    fn capabilities(&self) -> Capabilities;

    /// Direct children of `dir` (no `.` / `..`), hidden entries included
    /// and flagged.
    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>>;

    /// Metadata of `path`, without following a final symbolic link.
    fn stat(&self, path: &Path) -> io::Result<VfsEntry>;

    /// A stream over the contents of the file at `path`.
    fn open_read(&self, _path: &Path) -> io::Result<Box<dyn Read + Send>> {
        Err(unsupported())
    }

    /// Creates or replaces the file at `path` with `data`.
    fn write_file(&self, _path: &Path, _data: &mut dyn Read) -> io::Result<()> {
        Err(unsupported())
    }

    /// Creates the folder `path` (its parent must exist).
    fn mkdir(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    fn remove_file(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    /// Removes the empty folder `path`.
    fn remove_dir(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    /// Creates `dir` and its missing parents. Failures are ignored: the
    /// write that needs the folder reports them.
    fn mkdir_all(&self, dir: &Path) {
        let mut current = PathBuf::new();
        for component in dir.components() {
            current.push(component);
            if !self.exists(&current) {
                let _ = self.mkdir(&current);
            }
        }
    }

    /// `true` when an entry (of any kind, links not followed) is at `path`.
    fn exists(&self, path: &Path) -> bool {
        self.stat(path).is_ok()
    }

    /// Deletes `path` and everything below it, children before parents, so
    /// `remove_dir` only ever runs on an emptied folder. Links are removed,
    /// never followed. A missing `path` is not an error.
    fn remove_all(&self, path: &Path) -> io::Result<()> {
        match self.stat(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
            Ok(entry) if !entry.is_real_dir() => return self.remove_file(path),
            Ok(_) => {}
        }
        let mut stack: Vec<(PathBuf, bool)> = vec![(path.to_path_buf(), false)];
        while let Some((dir, expanded)) = stack.pop() {
            if expanded {
                self.remove_dir(&dir)?;
                continue;
            }
            stack.push((dir.clone(), true));
            for child in self.list(&dir)? {
                if child.is_real_dir() {
                    stack.push((child.path, false));
                } else {
                    self.remove_file(&child.path)?;
                }
            }
        }
        Ok(())
    }

    /// Every entry below `root` (depth first, links not followed).
    /// Unreadable folders are skipped.
    fn walk(&self, root: &Path) -> Vec<VfsEntry> {
        let mut out = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in self.list(&dir).unwrap_or_default() {
                if entry.is_real_dir() {
                    pending.push(entry.path.clone());
                }
                out.push(entry);
            }
        }
        out
    }

    /// The first `max` bytes of the file at `path`.
    fn read_prefix(&self, path: &Path, max: u64) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.open_read(path)?.take(max).read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Random-access bytes of the file at `path` for the viewer. Files
    /// larger than `cap` are refused unless the adapter pages them.
    fn open_store(&self, path: &Path, cap: u64) -> io::Result<Arc<dyn ByteStore>> {
        let size = self.stat(path)?.size;
        if size > cap {
            return Err(io::Error::other(
                crate::config::localization::t("vfs_file_too_large")
                    .replacen("{}", &bytesize::ByteSize::b(size).to_string(), 1)
                    .replacen("{}", &bytesize::ByteSize::b(cap).to_string(), 1),
            ));
        }
        let bytes = self.read_prefix(path, cap)?;
        Ok(Arc::new(crate::fs::text::store::MemStore::new(bytes)))
    }

    /// The panel listing of `dir`: `..`, hidden filter and sort as `opts` say.
    fn read_panel(&self, dir: &Path, opts: &ListOptions) -> anyhow::Result<Vec<FileEntry>> {
        Ok(panel_listing(dir, self.list(dir)?, opts))
    }

    /// Children of `dir` for the folder size walker (links not followed).
    fn du_list(&self, dir: &Path) -> io::Result<Vec<DuEntry>> {
        Ok(self
            .list(dir)?
            .into_iter()
            .map(|e| {
                let is_dir = e.is_real_dir();
                DuEntry {
                    kind: if is_dir { DuKind::Dir } else { DuKind::File },
                    size: if is_dir { 0 } else { e.size },
                    name: e.name,
                    path: e.path,
                    id: None,
                }
            })
            .collect())
    }
}
