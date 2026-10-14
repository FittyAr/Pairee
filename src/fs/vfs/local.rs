//! The local filesystem adapter.

use super::{Capabilities, Vfs, VfsEntry};
use crate::fs::FileEntry;
use crate::fs::attrs::{AttrChange, FileAttrs};
use crate::fs::file_id::{FileIdentity, file_identity};
use crate::fs::list::{ListOptions, is_hidden};
use crate::fs::remove::remove_entry;
use crate::fs::text::{ByteStore, FileStore};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVfs;

/// A file whose `flush` also syncs it to the disk.
struct SyncOnFlush(fs::File);

impl Write for SyncOnFlush {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        self.0.sync_all()
    }
}

/// Entry for `path` from its link metadata; links are followed for the
/// folder flag, size and time, so a link to a folder lists (and is
/// entered) as a folder. The link's own size, hard-link identity and
/// unreadable metadata are recorded for walkers that need them.
fn local_entry(name: String, path: std::path::PathBuf, link: Option<fs::Metadata>) -> VfsEntry {
    let hidden = is_hidden(&name, link.as_ref());
    let is_symlink = link.as_ref().is_some_and(|m| m.file_type().is_symlink());
    let link_size = link.as_ref().filter(|_| is_symlink).map(fs::Metadata::len);
    let hard_link = link.as_ref().filter(|m| m.is_file()).and_then(hard_link_id);
    let unreadable = link.is_none();
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
        link_size,
        hard_link,
        unreadable,
    }
}

/// `(device, inode)` of a file with more than one hard link.
#[cfg(unix)]
fn hard_link_id(meta: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    (meta.nlink() > 1).then(|| (meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn hard_link_id(_meta: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

impl Vfs for LocalVfs {
    fn capabilities(&self) -> Capabilities {
        Capabilities::FULL
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        self.list_reporting(dir).map(|(entries, _)| entries)
    }

    fn list_reporting(&self, dir: &Path) -> io::Result<(Vec<VfsEntry>, Option<io::Error>)> {
        let mut entries = Vec::new();
        let mut skipped = None;
        for entry in fs::read_dir(dir)? {
            match entry {
                Ok(entry) => {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    entries.push(local_entry(name, entry.path(), entry.metadata().ok()));
                }
                Err(e) => {
                    skipped.get_or_insert(e);
                }
            }
        }
        Ok((entries, skipped))
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

    fn metadata(&self, path: &Path) -> io::Result<VfsEntry> {
        // Fails on a broken link (unlike the listing, which keeps it).
        fs::metadata(path)?;
        let name = crate::fs::file_name_lossy(path);
        Ok(local_entry(
            name,
            path.to_path_buf(),
            Some(fs::symlink_metadata(path)?),
        ))
    }

    fn identity(&self, path: &Path) -> Option<FileIdentity> {
        file_identity(path, true).ok()
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let mut file = fs::File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        Ok(Box::new(file))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        Ok(Box::new(SyncOnFlush(fs::File::create(path)?)))
    }

    fn append_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let file = fs::OpenOptions::new().append(true).open(path)?;
        Ok(Box::new(SyncOnFlush(file)))
    }

    /// A read-only file is replaced too; a folder never is.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        crate::fs::transfer::part::commit(from, to, false).map(|_| ())
    }

    fn set_metadata(
        &self,
        path: &Path,
        modified: Option<SystemTime>,
        mode: Option<u32>,
    ) -> io::Result<()> {
        if let Some(time) = modified {
            filetime::set_file_mtime(path, filetime::FileTime::from_system_time(time))?;
        }
        #[cfg(unix)]
        if let Some(mode) = mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o7777))?;
        }
        #[cfg(not(unix))]
        let _ = mode;
        Ok(())
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn mkdir_all(&self, dir: &Path) {
        let _ = fs::create_dir_all(dir);
    }

    /// Files and links (folder links too); read-only entries as well.
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        remove_entry(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        remove_entry(path)
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
}
