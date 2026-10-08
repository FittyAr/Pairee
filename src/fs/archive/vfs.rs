//! The archive adapter of the panel source port: the inside of a zip, tar
//! or 7z file browsed as folders. Paths are `archive.ext/inner/path`.
//!
//! The folder tree is read once and cached until the archive file changes
//! (size or modification time). Entries are read into memory with a size
//! cap; zip archives can also be edited (see [`super::zip_write`]). An
//! archive inside an archive is browsed read-only from a temporary copy
//! (see [`super::nested`]).

use super::format::{ArchiveReader, Visit};
use super::index::{ArchiveIndex, Node};
use super::nested::ArchiveData;
use super::safe_extract::ExtractGuard;
use super::zip_write::{ZipEdit, ZipSource, rewrite_zip};
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry, unsupported};
use crate::lock::LockExt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Largest entry read into memory (viewer, quick view, copies between
/// non-local panels). Copying out of an archive extracts without a cap.
pub const MAX_ENTRY_READ_BYTES: u64 = 64 * 1024 * 1024;

/// Size and modification time of the archive file the cache was built from.
type Stamp = (u64, Option<SystemTime>);

pub struct ArchiveVfs {
    /// Path of the archive in the panel's namespace (its entries are below).
    root: PathBuf,
    data: ArchiveData,
    reader: &'static dyn ArchiveReader,
    cache: Mutex<Option<(Stamp, Arc<ArchiveIndex>)>>,
}

impl std::fmt::Debug for ArchiveVfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchiveVfs")
            .field("root", &self.root)
            .finish()
    }
}

fn to_io(e: anyhow::Error) -> io::Error {
    match e.downcast::<io::Error>() {
        Ok(io) => io,
        Err(other) => io::Error::other(other.to_string()),
    }
}

impl ArchiveVfs {
    /// The archive at `root`, or `None` when its format cannot be browsed.
    pub fn open(root: PathBuf) -> Option<Self> {
        let reader = super::browsable_reader(&root)?;
        Some(Self {
            data: ArchiveData::Local(root.clone()),
            root,
            reader,
            cache: Mutex::new(None),
        })
    }

    /// The archive stored as the entry `entry` of `parent` (read-only; it
    /// is extracted the first time it is read).
    pub fn open_nested(parent: Arc<ArchiveVfs>, entry: PathBuf) -> Option<Self> {
        let reader = super::browsable_reader(&entry)?;
        Some(Self {
            data: ArchiveData::nested(parent, entry.clone()),
            root: entry,
            reader,
            cache: Mutex::new(None),
        })
    }

    /// The archive that holds this one, when it is nested.
    pub fn parent(&self) -> Option<&Arc<ArchiveVfs>> {
        self.data.parent()
    }

    /// The nested archive `path` lies in (the shallowest archive file
    /// entry on the way from this archive's root to `path`), if any.
    pub fn nested_at(self: &Arc<Self>, path: &Path) -> Option<Self> {
        let mut inside: Vec<&Path> = path
            .ancestors()
            .take_while(|a| *a != self.root && a.starts_with(&self.root))
            .collect();
        inside.reverse();
        let entry = inside
            .into_iter()
            .find(|a| super::is_browsable(a) && self.stat(a).is_ok_and(|e| !e.is_dir))?;
        Self::open_nested(Arc::clone(self), entry.to_path_buf())
    }

    /// Path of the archive file (the root of its namespace).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `path` lies inside this archive (or is its root).
    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    /// The path of `path` inside the archive.
    pub fn inner(&self, path: &Path) -> io::Result<PathBuf> {
        path.strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .map_err(|_| io::Error::from(io::ErrorKind::NotFound))
    }

    fn index(&self) -> io::Result<Arc<ArchiveIndex>> {
        let file = self.data.file()?;
        let meta = std::fs::metadata(&file)?;
        let stamp = (meta.len(), meta.modified().ok());
        let mut cache = self.cache.lock_safe();
        if let Some((seen, index)) = cache.as_ref()
            && *seen == stamp
        {
            return Ok(Arc::clone(index));
        }
        let entries = self.reader.entries(&file).map_err(to_io)?;
        let index = Arc::new(ArchiveIndex::build(&entries));
        *cache = Some((stamp, Arc::clone(&index)));
        Ok(index)
    }

    fn node(&self, path: &Path) -> io::Result<(PathBuf, Node)> {
        let inner = self.inner(path)?;
        let node = self
            .index()?
            .node(&inner)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok((inner, node))
    }

    fn entry(&self, name: String, path: PathBuf, node: Node) -> VfsEntry {
        VfsEntry {
            hidden: name.starts_with('.'),
            name,
            path,
            is_dir: node.is_dir,
            is_symlink: node.is_symlink,
            size: node.size,
            modified: node.modified,
        }
    }

    /// Writes the first `max` bytes of the entry at `inner` to `out`;
    /// returns how many were written.
    fn copy_inner(&self, inner: &Path, out: &mut dyn Write, max: u64) -> io::Result<u64> {
        let mut written = None;
        self.reader
            .visit(&self.data.file()?, &mut |meta, data| {
                if ExtractGuard::sanitize(&meta.name).ok().as_deref() != Some(inner) {
                    return Ok(Visit::Continue);
                }
                written = Some(io::copy(&mut data.take(max), out)?);
                Ok(Visit::Stop)
            })
            .map_err(to_io)?;
        written.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    /// Streams the first `max` bytes of the entry at `path` to `out`.
    pub fn copy_entry(&self, path: &Path, out: &mut dyn Write, max: u64) -> io::Result<u64> {
        self.copy_inner(&self.inner(path)?, out, max)
    }

    /// The first `max` bytes of the entry at `inner`.
    fn read_entry(&self, inner: &Path, max: u64) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.copy_inner(inner, &mut buf, max)?;
        Ok(buf)
    }

    /// Entries can be added and removed (a local zip file).
    fn writable(&self) -> bool {
        self.reader.writable() && self.parent().is_none()
    }

    /// Rewrites a writable archive with `edit`.
    pub fn edit(
        &self,
        edit: &ZipEdit,
        on_added: &mut dyn FnMut(&Path) -> io::Result<()>,
    ) -> io::Result<()> {
        if !self.writable() {
            return Err(unsupported());
        }
        let result = rewrite_zip(&self.root, edit, on_added).map_err(to_io);
        // The rewrite may keep the archive's size and time stamp.
        *self.cache.lock_safe() = None;
        result
    }

    fn edit_one(&self, edit: ZipEdit) -> io::Result<()> {
        self.edit(&edit, &mut |_| Ok(()))
    }
}

impl Vfs for ArchiveVfs {
    fn capabilities(&self) -> Capabilities {
        let writable = self.writable();
        Capabilities {
            write: writable,
            mkdir: writable,
            remove: writable,
            ..Capabilities::READ_ONLY
        }
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let inner = self.inner(dir)?;
        let index = self.index()?;
        let children = index
            .children(&inner)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let base = self.root.join(&inner);
        Ok(children
            .iter()
            .map(|(name, node)| self.entry(name.clone(), base.join(name), node.clone()))
            .collect())
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let (_, node) = self.node(path)?;
        let name = crate::fs::file_name_lossy(path);
        Ok(self.entry(name, path.to_path_buf(), node))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        let entry = self.stat(path)?;
        if entry.size > MAX_ENTRY_READ_BYTES {
            return Err(io::Error::other(
                crate::config::localization::t("vfs_file_too_large")
                    .replacen("{}", &bytesize::ByteSize::b(entry.size).to_string(), 1)
                    .replacen(
                        "{}",
                        &bytesize::ByteSize::b(MAX_ENTRY_READ_BYTES).to_string(),
                        1,
                    ),
            ));
        }
        let bytes = self.read_entry(&self.inner(path)?, MAX_ENTRY_READ_BYTES)?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    fn read_prefix(&self, path: &Path, max: u64) -> io::Result<Vec<u8>> {
        self.read_entry(&self.inner(path)?, max)
    }

    fn write_file(&self, path: &Path, data: &mut dyn Read) -> io::Result<()> {
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        self.edit_one(ZipEdit {
            add: vec![(self.inner(path)?, ZipSource::Bytes(bytes))],
            ..ZipEdit::default()
        })
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.edit_one(ZipEdit {
            add: vec![(self.inner(path)?, ZipSource::Dir)],
            ..ZipEdit::default()
        })
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.remove_all(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.remove_all(path)
    }

    /// One rewrite for the whole subtree.
    fn remove_all(&self, path: &Path) -> io::Result<()> {
        let inner = self.inner(path)?;
        if inner.as_os_str().is_empty() {
            return Err(unsupported());
        }
        self.edit_one(ZipEdit {
            remove: vec![inner],
            ..ZipEdit::default()
        })
    }
}
