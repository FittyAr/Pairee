//! Folder comparison: matching the entries of two local directories and
//! deciding whether each pair is equal (size + modification time within a
//! tolerance).

mod key;
mod summary;
#[cfg(test)]
mod tests;

use key::platform_case_insensitive;
use summary::{EntryPair, metadata_equal, pair_entries, scan_directory};

use anyhow::Result;
use std::path::Path;
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

/// Compares the entries of two directories (one level) by name, size and
/// modification time. Results are sorted by name key.
pub fn compare_directories(
    left: &Path,
    right: &Path,
    options: &CompareOptions,
) -> Result<Vec<CompareEntry>> {
    let ci = options.case_insensitive;
    let left_map = scan_directory(left, ci, |_, _| true)?;
    let right_map = scan_directory(right, ci, |_, _| true)?;
    Ok(pair_entries(left_map, right_map)
        .into_iter()
        .map(|pair| {
            let status = match &pair {
                EntryPair::Left(_) => CompareStatus::OnlyLeft,
                EntryPair::Right(_) => CompareStatus::OnlyRight,
                EntryPair::Both(l, r)
                    if metadata_equal(&l.summary, &r.summary, options.mtime_tolerance) =>
                {
                    CompareStatus::Equal
                }
                EntryPair::Both(..) => CompareStatus::Different,
            };
            CompareEntry {
                name: pair.name().to_owned(),
                status,
            }
        })
        .collect())
}
