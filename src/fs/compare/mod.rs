//! Folder comparison primitives: reading one directory level, matching the
//! entries of both sides by name (ignoring case where the platform does)
//! and deciding whether a pair is equal (size + modification time within a
//! tolerance). The recursive walk lives in [`crate::fs::sync`].

mod key;
mod summary;
#[cfg(test)]
mod tests;

pub use key::platform_case_insensitive;
pub use summary::{
    EntryPair, FileSummary, ScannedDir, ScannedEntry, metadata_equal, mtime_within, pair_entries,
    scan_directory,
};

use std::time::Duration;

/// Default modification-time tolerance: FAT stores times with a 2-second
/// granularity, so copies to/from FAT drives differ by up to 2 s.
pub const DEFAULT_MTIME_TOLERANCE_SECS: u64 = 2;

/// How entries of both sides are matched and judged equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareOptions {
    /// Largest modification-time difference still considered equal.
    pub mtime_tolerance: Duration,
    /// Match names ignoring case (Windows / macOS).
    pub case_insensitive: bool,
}

impl Default for CompareOptions {
    fn default() -> Self {
        Self {
            mtime_tolerance: Duration::from_secs(DEFAULT_MTIME_TOLERANCE_SECS),
            case_insensitive: platform_case_insensitive(),
        }
    }
}

impl CompareOptions {
    /// Options from the user settings (`compare_mtime_tolerance_secs`).
    pub fn from_settings(settings: &crate::config::settings::Settings) -> Self {
        Self {
            mtime_tolerance: Duration::from_secs(settings.compare_mtime_tolerance_secs),
            ..Self::default()
        }
    }
}

/// Status of a file relative to the two panels being compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareStatus {
    /// File only exists in the left panel directory.
    OnlyLeft,
    /// File only exists in the right panel directory.
    OnlyRight,
    /// File exists in both but differs in size or modification time.
    Different,
    /// File exists in both and appears identical (same size + mtime).
    Equal,
}

/// One entry in the comparison result.
#[derive(Debug, Clone)]
pub struct CompareEntry {
    /// Name as shown (the left-side spelling when both sides have it).
    pub name: String,
    pub status: CompareStatus,
}
