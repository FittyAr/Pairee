//! Recursive directory sizes (folder size calculation and the disk usage
//! view). No UI code: the app layer runs [`scan`] in a background job.
//!
//! * Symbolic links (and Windows junctions) are never followed: a link counts
//!   as a small file of its own size.
//! * On Unix a file with several hard links is counted once per scan
//!   (`(dev, ino)` pairs are remembered).
//! * Unreadable directories and entries do not abort the scan: the result is
//!   flagged [`DirSize::partial`] and the UI shows it with a marker.
//!
//! Directory listing goes through the panel source port
//! ([`crate::fs::vfs::Vfs::du_list`]), so the same walker serves local,
//! SFTP and archive panels.

mod node;
mod scan;
mod source;

#[cfg(test)]
mod tests;

pub use node::DuNode;
pub use scan::{ScanControl, scan};
pub use source::{DuEntry, DuKind, local_entries};

use std::path::PathBuf;

/// Accumulated size of a directory tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirSize {
    /// Apparent size of the files, in bytes (hard links counted once).
    pub bytes: u64,
    /// Number of files (including symbolic links).
    pub files: u64,
    /// Number of subdirectories (the directory itself is not counted).
    pub dirs: u64,
    /// Something could not be read or the scan was cancelled: the values
    /// are a lower bound.
    pub partial: bool,
}

impl DirSize {
    /// Adds `other` to `self` (sizes, counts and the partial flag).
    pub fn add(&mut self, other: &DirSize) {
        self.bytes = self.bytes.saturating_add(other.bytes);
        self.files = self.files.saturating_add(other.files);
        self.dirs = self.dirs.saturating_add(other.dirs);
        self.partial |= other.partial;
    }

    /// Removes `other` from `self` (after deleting a subtree).
    pub fn sub(&mut self, other: &DirSize) {
        self.bytes = self.bytes.saturating_sub(other.bytes);
        self.files = self.files.saturating_sub(other.files);
        self.dirs = self.dirs.saturating_sub(other.dirs);
    }
}

/// Running totals published while a scan is in progress.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanProgress {
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
    /// Directory being read.
    pub current: PathBuf,
}
