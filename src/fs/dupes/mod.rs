//! Duplicate file finder (FEAT-09 core; no UI code: the app layer runs
//! [`find_duplicates`] in a background job).
//!
//! The candidates narrow in three passes, each cheaper than the next:
//!
//! 1. **size** — the trees are read by the shared walker
//!    ([`crate::fs::walk::Walker`]) over any [`Vfs`]; only files sharing a
//!    size go on. Links are never followed and hard links of one file
//!    (the identity folder sizes use, [`crate::fs::vfs::VfsEntry::hard_link`])
//!    count once: they take no extra space;
//! 2. **partial hash** — the first [`DupeOptions::partial_bytes`] of each
//!    candidate;
//! 3. **full hash** — the whole file, with the transfer engine's hashers.
//!
//! Every pass polls the cancel callback; a cancelled or partly unreadable
//! search returns what it found, flagged [`DupeReport::partial`].

mod group;
pub mod keep;
mod scan;

#[cfg(test)]
mod tests;

use crate::fs::transfer::options::HashAlgorithm;
use crate::fs::vfs::Vfs;
use std::path::PathBuf;

/// How the search runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DupeOptions {
    /// Smaller files are left out (empty files are all "equal").
    pub min_size: u64,
    /// Bytes hashed by the partial pass.
    pub partial_bytes: u64,
    /// Hash of the full pass.
    pub algorithm: HashAlgorithm,
    /// Follow symbolic links (and junctions) into folders and files; a
    /// file reached twice (through a link and its own path) counts once,
    /// and links that loop back are skipped.
    pub follow_links: bool,
}

impl Default for DupeOptions {
    fn default() -> Self {
        Self {
            min_size: 1,
            partial_bytes: 64 * 1024,
            algorithm: HashAlgorithm::Blake3,
            follow_links: false,
        }
    }
}

/// Files with the same contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DupeGroup {
    /// Size of each file.
    pub size: u64,
    /// Hash of the contents (full pass).
    pub hash: String,
    /// The files, sorted by path.
    pub files: Vec<PathBuf>,
}

impl DupeGroup {
    /// Bytes that deleting all copies but one would free.
    pub fn wasted(&self) -> u64 {
        self.size
            .saturating_mul(self.files.len().saturating_sub(1) as u64)
    }
}

/// What a search found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DupeReport {
    /// Groups of two or more files, most wasted space first.
    pub groups: Vec<DupeGroup>,
    /// Folders or files that could not be read.
    pub unreadable: Vec<PathBuf>,
    /// Cancelled, or something could not be read: there may be more.
    pub partial: bool,
}

/// Which pass a progress report is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupeStage {
    Scanning,
    PartialHash,
    FullHash,
}

/// Running totals published while a search is in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DupeProgress {
    pub stage: DupeStage,
    /// Files seen (scanning) or hashed (hash passes) so far.
    pub done: u64,
    /// Files the pass has to hash (0 while scanning).
    pub total: u64,
    /// Entry being read.
    pub current: PathBuf,
}

/// Cancellation and progress hooks of [`find_duplicates`].
pub struct DupeControl<'a> {
    /// Polled between entries and blocks.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Receives running totals (rate-limited).
    pub progress: &'a dyn Fn(&DupeProgress),
}

/// Finds the files below `roots` (on `vfs`) that have the same contents.
pub fn find_duplicates(
    vfs: &dyn Vfs,
    roots: &[PathBuf],
    options: &DupeOptions,
    ctl: &DupeControl,
) -> DupeReport {
    let mut report = DupeReport::default();
    let by_size = scan::files_by_size(vfs, roots, options, ctl, &mut report);
    let mut hasher = group::Hasher::new(vfs, options, ctl, &mut report);
    let partial = hasher.split(by_size, DupeStage::PartialHash);
    let full = hasher.split(partial, DupeStage::FullHash);
    let mut groups = group::finish(full);
    groups.sort_by(|a, b| {
        b.wasted()
            .cmp(&a.wasted())
            .then_with(|| a.files.cmp(&b.files))
    });
    report.groups = groups;
    report
}
