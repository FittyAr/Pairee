//! What a synchronization compares and which entries take part.

use super::model::SyncDirection;
use crate::fs::compare::{CompareOptions, ScannedEntry};
use crate::fs::transfer::filter::TransferFilter;
use crate::fs::transfer::options::HashAlgorithm;

/// Glob that matches dot-files (added to the mask when hidden files are
/// ignored, so the copy jobs skip them too).
const HIDDEN_GLOB: &str = ".*";

/// Options of one comparison run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOptions {
    pub compare: CompareOptions,
    /// Compare same-size files by content with this hash.
    pub content_hash: Option<HashAlgorithm>,
    /// Transfer filter syntax: `*.rs;*.toml` include, `!target` exclude
    /// (also size/date rules), `;`-separated.
    pub mask: String,
    pub ignore_hidden: bool,
    pub direction: SyncDirection,
}

impl SyncOptions {
    /// The mask handed to the copy jobs (and used for the comparison).
    pub fn effective_mask(&self) -> String {
        let mask = self.mask.trim();
        match (self.ignore_hidden, mask.is_empty()) {
            (false, _) => mask.to_owned(),
            (true, true) => format!("!{HIDDEN_GLOB}"),
            (true, false) => format!("{mask};!{HIDDEN_GLOB}"),
        }
    }

    pub fn filter(&self) -> SyncFilter {
        SyncFilter {
            rules: TransferFilter::parse(&self.effective_mask()),
            ignore_hidden: self.ignore_hidden,
        }
    }
}

/// Decides which scanned entries take part in the comparison.
#[derive(Debug, Clone)]
pub struct SyncFilter {
    rules: TransferFilter,
    ignore_hidden: bool,
}

impl SyncFilter {
    /// Folders are only dropped by exclusions; files by every rule.
    pub fn accepts(&self, entry: &ScannedEntry, meta: &std::fs::Metadata) -> bool {
        if self.ignore_hidden && crate::fs::list::is_hidden(&entry.name, Some(meta)) {
            return false;
        }
        if entry.summary.is_dir {
            !self.rules.excludes(&entry.name)
        } else {
            self.rules.matches(&entry.path, entry.summary.size)
        }
    }
}
