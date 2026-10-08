//! The local filesystem adapter.

use super::{Capabilities, Vfs, VfsEntry};
use crate::fs::FileEntry;
use crate::fs::attrs::{AttrChange, FileAttrs};
use crate::fs::du::DuEntry;
use crate::fs::list::{ListOptions, is_hidden};
use crate::fs::text::{ByteStore, FileStore};
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVfs;

/// Entry for `path` from its link metadata; links are followed for the
/// folder flag, size and time, so a link to a folder lists (and is
/// entered) as a folder.
fn local_entry(name: String, path: std::path::PathBuf, link: Option<fs::Metadata>) -> VfsEntry {
    let hidden = is_hidden(&name, link.as_ref());
    let is_symlink = link.as_ref().is_some_and(|m| m.file_type().is_symlink());
    let meta = if is_symlink {
        fs::metadata(&path).ok().or(link)
    } else {
        link
    };
    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
    VfsEntry {
        name,
        path,
        is_dir,
        is_symlink,
        size: meta.as_ref().map_or(0, fs::Metadata::len),
        modified: meta.and_then(|m| m.modified().ok()),
        hidden,
    }
}

impl Vfs for LocalVfs {
    fn capabilities(&self) -> Capabilities {
        Capabilities::FULL
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        Ok(fs::read_dir(dir)?
            .flatten()
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                local_entry(name, entry.path(), entry.metadata().ok())
            })
            .collect())
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let meta = fs::symlink_metadata(path)?;
        let name = crate::fs::file_name_lossy(path);
        let mut entry = local_entry(name, path.to_path_buf(), Some(meta));
        // `stat` does not follow a final link.
        if entry.is_symlink {
            entry.is_dir = false;
        }
        Ok(entry)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    fn write_file(&self, path: &Path, data: &mut dyn Read) -> io::Result<()> {
        io::copy(data, &mut fs::File::create(path)?).map(|_| ())
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn mkdir_all(&self, dir: &Path) {
        let _ = fs::create_dir_all(dir);
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn attributes(&self, path: &Path) -> io::Result<FileAttrs> {
        crate::fs::attrs::read_attrs(path).map_err(|e| io::Error::other(e.to_string()))
    }

    fn set_attributes(&self, path: &Path, change: AttrChange) -> io::Result<()> {
        change.apply_local(path)
    }

    /// Local files are paged from disk, whatever their size.
    fn open_store(&self, path: &Path, _cap: u64) -> io::Result<Arc<dyn ByteStore>> {
        Ok(Arc::new(FileStore::open(path)?))
    }

    /// Adds the elevated retry of unreadable folders.
    fn read_panel(&self, dir: &Path, opts: &ListOptions) -> anyhow::Result<Vec<FileEntry>> {
        crate::fs::list::read_directory_ext(dir, opts)
    }

    /// Keeps hard-link identities (counted once) and unreadable entries.
    fn du_list(&self, dir: &Path) -> io::Result<Vec<DuEntry>> {
        crate::fs::du::local_entries(dir)
    }
}
