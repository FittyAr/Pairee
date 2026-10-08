//! One directory level read into comparable summaries, keyed by name.

use super::key::name_key;
use std::collections::BTreeMap;
use std::fs::Metadata;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Size, modification time and kind of one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSummary {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub is_dir: bool,
}

impl FileSummary {
    pub fn from_metadata(meta: &Metadata) -> Self {
        Self {
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified: meta.modified().ok(),
            is_dir: meta.is_dir(),
        }
    }
}

/// An entry of a scanned directory: its real name and path plus summary.
#[derive(Debug, Clone)]
pub struct ScannedEntry {
    pub name: String,
    pub summary: FileSummary,
}

/// Entries of one directory keyed by [`name_key`] (sorted by key).
pub type ScannedDir = BTreeMap<String, ScannedEntry>;

/// Reads `dir` (one level). `keep` decides, from the entry and its metadata,
/// whether the entry takes part in the comparison (filters, hidden files).
pub fn scan_directory(
    dir: &Path,
    case_insensitive: bool,
    keep: impl Fn(&ScannedEntry, &Metadata) -> bool,
) -> std::io::Result<ScannedDir> {
    let mut map = ScannedDir::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let scanned = ScannedEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            summary: FileSummary::from_metadata(&meta),
        };
        if keep(&scanned, &meta) {
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
