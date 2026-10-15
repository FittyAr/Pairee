//! Panel source port (Ports & Adapters).
//!
//! [`Vfs`] is the one interface panels, folder sizes, multi-rename, compare,
//! the viewer and quick view use to read (and, when the adapter can, change)
//! the place a panel shows. Adapters:
//!
//! * [`LocalVfs`] — the local filesystem;
//! * `SharedSshClient` (in `crate::fs::ssh`) — an SFTP server;
//! * `FtpVfs`, `WebDavVfs`, `S3Vfs` (in [`crate::fs::remote`]) — FTP / FTPS,
//!   WebDAV and S3 servers (panels hold them as a `RemoteConn`);
//! * `crate::fs::archive::ArchiveVfs` — the inside of a zip / tar / 7z file.
//!
//! Every adapter reports what it supports through [`Capabilities`]; the
//! operations it does not support return [`unsupported`] errors, so callers
//! check the flags first and show a message instead of failing half-way.

mod entry;
mod local;
mod source;

#[cfg(test)]
pub(crate) mod contract;

pub use entry::{VfsEntry, is_plain_name, panel_listing, parent_entry};
pub use local::LocalVfs;
pub use source::PanelSource;

use crate::fs::FileEntry;
use crate::fs::attrs::{AttrChange, FileAttrs};
use crate::fs::file_id::FileIdentity;
use crate::fs::list::ListOptions;
use crate::fs::text::ByteStore;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

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
    /// Read and change permissions ([`Vfs::attributes`]).
    pub attributes: bool,
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
        attributes: false,
        local_tools: false,
    };

    /// Everything (the local filesystem).
    pub const FULL: Self = Self {
        write: true,
        mkdir: true,
        remove: true,
        rename: true,
        attributes: true,
        local_tools: true,
    };

    pub fn allows(self, capability: Capability) -> bool {
        match capability {
            Capability::Write => self.write,
            Capability::MkDir => self.mkdir,
            Capability::Remove => self.remove,
            Capability::Rename => self.rename,
            Capability::Attributes => self.attributes,
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
    Attributes,
    LocalTools,
}

/// The error an adapter returns for an operation it does not support.
pub fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        crate::config::localization::t("vfs_unsupported_operation"),
    )
}

/// `Err` unless the folder `dir` of `vfs` is empty: for adapters whose
/// folder deletion takes the contents with it (WebDAV `DELETE`, archive
/// rewrites), so [`Vfs::remove_dir`] keeps its "empty folders only" promise.
pub fn ensure_empty_dir(vfs: &dyn Vfs, dir: &Path) -> io::Result<()> {
    if vfs.list(dir)?.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::DirectoryNotEmpty,
            dir.to_string_lossy().into_owned(),
        ))
    }
}

/// [`Vfs::replace`] for servers whose rename refuses an existing target
/// (SFTP v3, some FTP servers): the old file is moved aside, the new one
/// put in place and the old one deleted — or put back when the second
/// rename fails, so the destination is never lost. The name it is moved to
/// is never one that exists ([`free_aside_name`]): a copy left by an earlier
/// interrupted replace may be the only one of the user's file, so it is
/// neither overwritten nor deleted.
pub fn replace_by_moving_aside(vfs: &dyn Vfs, from: &Path, to: &Path) -> io::Result<()> {
    let first = match vfs.rename(from, to) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    if !vfs.stat(to).is_ok_and(|e| !e.is_dir) {
        return Err(first);
    }
    let aside = free_aside_name(vfs, to)?;
    vfs.rename(to, &aside)?;
    match vfs.rename(from, to) {
        Ok(()) => {
            let _ = vfs.remove_file(&aside);
            Ok(())
        }
        Err(e) => {
            let _ = vfs.rename(&aside, to);
            Err(e)
        }
    }
}

/// `to` + `.pairee-old` (then `.pairee-old.1`, `.2`...): the first name
/// nothing exists at.
fn free_aside_name(vfs: &dyn Vfs, to: &Path) -> io::Result<PathBuf> {
    for attempt in 0..100u32 {
        let mut name = to.as_os_str().to_owned();
        name.push(".pairee-old");
        if attempt > 0 {
            name.push(format!(".{attempt}"));
        }
        let candidate = PathBuf::from(name);
        match vfs.stat(&candidate) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        to.to_string_lossy().into_owned(),
    ))
}

/// A browsable filesystem (the port). Paths are absolute in the adapter's
/// own namespace: local paths, remote paths, or `archive.zip/inner/path`.
pub trait Vfs: Send + Sync + std::fmt::Debug {
    fn capabilities(&self) -> Capabilities;

    /// Direct children of `dir` (no `.` / `..`), hidden entries included
    /// and flagged.
    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>>;

    /// Like [`Vfs::list`], plus the first error met while reading the
    /// entries (the listing itself worked but left some entries out).
    /// Walkers report it so totals are marked incomplete; adapters that
    /// list in one request never leave entries out.
    fn list_reporting(&self, dir: &Path) -> io::Result<(Vec<VfsEntry>, Option<io::Error>)> {
        Ok((self.list(dir)?, None))
    }

    /// Metadata of `path`, without following a final symbolic link.
    fn stat(&self, path: &Path) -> io::Result<VfsEntry>;

    /// Metadata of what `path` leads to (a final link is followed; a broken
    /// link is an error). Adapters without links use [`Vfs::stat`].
    fn metadata(&self, path: &Path) -> io::Result<VfsEntry> {
        self.stat(path)
    }

    /// Identity of the folder or file `path` leads to, used to detect link
    /// loops; `None` when the adapter cannot tell.
    fn identity(&self, _path: &Path) -> Option<FileIdentity> {
        None
    }

    /// A stream over the contents of the file at `path`.
    fn open_read(&self, _path: &Path) -> io::Result<Box<dyn Read + Send>> {
        Err(unsupported())
    }

    /// A stream over the contents of the file at `path` from byte `offset`.
    /// Adapters that can seek override this; the default reads and drops
    /// the first `offset` bytes.
    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let mut reader = self.open_read(path)?;
        let skipped = io::copy(&mut (&mut reader).take(offset), &mut io::sink())?;
        if skipped < offset {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        Ok(reader)
    }

    /// Creates (or truncates) the file `path` for writing. Flushing the
    /// writer makes its data durable (it reaches the disk or server).
    fn create_file(&self, _path: &Path) -> io::Result<Box<dyn Write + Send>> {
        Err(unsupported())
    }

    /// Opens the existing file `path` for writing after its last byte (an
    /// interrupted copy resumed). Flushing makes the data durable.
    fn append_file(&self, _path: &Path) -> io::Result<Box<dyn Write + Send>> {
        Err(unsupported())
    }

    /// Moves the file `from` over `to`, replacing a file there.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.rename(from, to)
    }

    /// Creates or replaces the file at `path` with `data`, atomically: the
    /// data goes to a temporary file that is flushed and then moved over
    /// `path`, so a failure never leaves a truncated file (nor loses the
    /// file being replaced).
    fn write_file(&self, path: &Path, data: &mut dyn Read) -> io::Result<()> {
        crate::fs::transfer::part::write_atomically(
            path,
            |part| {
                let mut out = self.create_file(part)?;
                io::copy(data, &mut out)?;
                out.flush()
            },
            |from, to| self.replace(from, to),
            |part| {
                let _ = self.remove_file(part);
            },
        )
    }

    /// Sets the modification time and/or the Unix permission bits of `path`.
    fn set_metadata(
        &self,
        _path: &Path,
        _modified: Option<SystemTime>,
        _mode: Option<u32>,
    ) -> io::Result<()> {
        Err(unsupported())
    }

    /// Creates the folder `path` (its parent must exist).
    fn mkdir(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    fn remove_file(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    /// Removes the empty folder `path`; a folder that still holds anything
    /// is an error (moves rely on it to keep what they skipped).
    fn remove_dir(&self, _path: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    /// Permissions, owner and times of `path` (links followed).
    fn attributes(&self, _path: &Path) -> io::Result<FileAttrs> {
        Err(unsupported())
    }

    /// Applies the Attributes dialog's `change` to `path`.
    fn set_attributes(&self, _path: &Path, _change: AttrChange) -> io::Result<()> {
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
        crate::fs::remove::remove_tree(self, path)
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
}
