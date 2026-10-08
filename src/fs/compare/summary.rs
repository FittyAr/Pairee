//! One directory level read into comparable summaries, keyed by name.

use super::key::name_key;
use crate::fs::vfs::{Vfs, VfsEntry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Size, modification time and kind of one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSummary {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub is_dir: bool,
}

impl FileSummary {
    pub fn from_entry(entry: &VfsEntry) -> Self {
        Self {
            size: if entry.is_dir { 0 } else { entry.size },
            modified: entry.modified,
            is_dir: entry.is_dir,
        }
    }
}

/// An entry of a scanned directory: its real name and path plus summary.
#[derive(Debug, Clone)]
pub struct ScannedEntry {
    pub name: String,
    pub path: PathBuf,
    /// The entry is a symbolic link (folders behind links are not entered).
    pub is_symlink: bool,
    pub summary: FileSummary,
}

/// Entries of one directory keyed by [`name_key`] (sorted by key).
pub type ScannedDir = BTreeMap<String, ScannedEntry>;

/// Reads `dir` (one level) on `vfs`. `keep` decides, from the entry and
/// whether it is hidden, if the entry takes part in the comparison.
pub fn scan_directory(
    vfs: &dyn Vfs,
    dir: &Path,
    case_insensitive: bool,
    keep: impl Fn(&ScannedEntry, bool) -> bool,
) -> std::io::Result<ScannedDir> {
    let mut map = ScannedDir::new();
    for entry in vfs.list(dir)? {
        let scanned = ScannedEntry {
            summary: FileSummary::from_entry(&entry),
            name: entry.name,
            path: entry.path,
            is_symlink: entry.is_symlink,
        };
        if keep(&scanned, entry.hidden) {
            map.insert(name_key(&scanned.name, case_insensitive), scanned);
        }
    }
    Ok(map)
}

/// Entries of both sides matched by name key.
#[derive(Debug, Clone)]
pub enum EntryPair {
    Left(ScannedEntry),
    Right(ScannedEntry),
    Both(ScannedEntry, ScannedEntry),
}

impl EntryPair {
    /// Display name (the left-side spelling when both sides have it).
    pub fn name(&self) -> &str {
        match self {
            Self::Left(e) | Self::Right(e) | Self::Both(e, _) => &e.name,
        }
    }
}

/// The entries of both sides matched by key, in key order.
pub fn pair_entries(left: ScannedDir, mut right: ScannedDir) -> Vec<EntryPair> {
    let mut pairs: BTreeMap<String, EntryPair> = left
        .into_iter()
        .map(|(key, l)| {
            let pair = match right.remove(&key) {
                Some(r) => EntryPair::Both(l, r),
                None => EntryPair::Left(l),
            };
            (key, pair)
        })
        .collect();
    pairs.extend(right.into_iter().map(|(key, r)| (key, EntryPair::Right(r))));
    pairs.into_values().collect()
}

/// True when both times are known and at most `tolerance` apart, or both
/// are unknown.
pub fn mtime_within(a: Option<SystemTime>, b: Option<SystemTime>, tolerance: Duration) -> bool {
    match (a, b) {
        (Some(ta), Some(tb)) => {
            let delta = ta.duration_since(tb).unwrap_or_else(|e| e.duration());
            delta <= tolerance
        }
        (None, None) => true,
        _ => false,
    }
}

/// Same kind, and for files same size and modification time (within
/// `tolerance`).
pub fn metadata_equal(l: &FileSummary, r: &FileSummary, tolerance: Duration) -> bool {
    l.is_dir == r.is_dir
        && (l.is_dir || (l.size == r.size && mtime_within(l.modified, r.modified, tolerance)))
}
