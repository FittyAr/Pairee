//! Views over the panels' files: the diff viewer shared by the Git diff and
//! Compare files, the folder jump list and the duplicate finder dialogs.

use crate::app::text_input::TextField;
use crate::diff::DiffDocument;
use crate::diff::files::BinaryComparison;
use crate::fs::transfer::options::HashAlgorithm;
use std::path::PathBuf;

/// A scrolled diff, unified or side by side (Git diff, Compare files).
#[derive(Debug, Clone, Default)]
pub struct DiffViewer {
    pub doc: DiffDocument,
    /// First row shown.
    pub scroll: usize,
    pub side_by_side: bool,
}

impl DiffViewer {
    pub fn new(doc: DiffDocument) -> Self {
        Self {
            doc,
            ..Self::default()
        }
    }

    /// Rows of the current layout.
    pub fn rows(&self) -> usize {
        if self.side_by_side {
            self.doc.side_by_side().len()
        } else {
            self.doc.lines.len()
        }
    }
}

/// Where a file comparison stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareStatus {
    Running,
    Failed(String),
    /// Text files: the diff is in the viewer.
    Text,
    Binary(BinaryComparison),
}

/// Compare files: the two files and, once computed, what differs.
#[derive(Debug, Clone)]
pub struct FileCompareView {
    pub old: PathBuf,
    pub new: PathBuf,
    pub status: CompareStatus,
    /// The text diff (empty for binary files).
    pub viewer: DiffViewer,
}

/// Folder jump: visited folders by frecency, filtered by a fuzzy query.
#[derive(Debug, Clone)]
pub struct FolderJump {
    pub query: TextField,
    pub cursor_idx: usize,
    /// Folders matching the query, best first.
    pub items: Vec<PathBuf>,
}

/// Duplicate finder options: where to search, smallest size and hash.
#[derive(Debug, Clone)]
pub struct DupeOptionsPrompt {
    /// Folders (or files) searched: the selection, or the current folder.
    pub roots: Vec<PathBuf>,
    /// Smallest file size searched ("1", "4 KiB", "10MB"...).
    pub min_size: TextField,
    pub algorithm: HashAlgorithm,
    pub cursor_idx: usize,
}

/// Duplicate finder results; the search and the marks live in
/// `AppState::dupes`.
#[derive(Debug, Clone)]
pub struct DuplicatesView;
