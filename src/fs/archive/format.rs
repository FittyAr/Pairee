//! Archive format strategies: every supported format reads its entries
//! through [`ArchiveReader`], so extraction, listing, the archive browser
//! and single-entry reads share one code path per operation instead of
//! one per format.

use anyhow::Result;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What an archive entry is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    /// A symbolic link to the given target (tar only).
    Symlink(PathBuf),
    /// Hard links, devices, FIFOs: never materialised.
    Other,
}

/// Metadata of one archive entry, as stored (the name is untrusted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMeta {
    pub name: String,
    pub kind: EntryKind,
    /// Uncompressed size declared by the archive.
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// Unix permission bits, when the format stores and applies them.
    pub mode: Option<u32>,
}

/// Whether [`ArchiveReader::visit`] goes on after an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visit {
    Continue,
    Stop,
}

/// Receives each entry with a reader over its data.
pub type Visitor<'a> = dyn FnMut(&EntryMeta, &mut dyn Read) -> Result<Visit> + 'a;

/// One archive format (Strategy).
pub trait ArchiveReader: Send + Sync {
    /// Every entry in archive order, without decompressing data when the
    /// format allows it.
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>>;

    /// Streams the entries in archive order to `visit` until it stops.
    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()>;

    /// Number of entries when it is known without reading the whole
    /// archive (progress totals); 0 when unknown.
    fn count_hint(&self, _archive: &Path) -> usize {
        0
    }

    /// The format can be rewritten in place (add, delete entries).
    fn writable(&self) -> bool {
        false
    }
}

/// Converts a timestamp in seconds since the Unix epoch.
pub(super) fn unix_time(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)
}
