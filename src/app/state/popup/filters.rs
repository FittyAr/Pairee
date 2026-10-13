//! One-field prompts on the panels (search, filters, tab title) and the
//! Find file dialog with its results.

use crate::app::text_input::TextField;
use std::path::PathBuf;

/// In-panel search (`find_in_panel`, NC quick search).
#[derive(Debug, Clone)]
pub struct PanelSearch {
    pub query: TextField,
    pub how: crate::app::input::panel_find::NameMatch,
    /// Cursor when the search started.
    pub origin: usize,
}

/// Persistent file mask of the active panel.
#[derive(Debug, Clone)]
pub struct FilePanelFilterPrompt {
    pub input: TextField,
}

/// Title of a folder tab (empty: the folder name).
#[derive(Debug, Clone)]
pub struct RenameTabPrompt {
    pub tab: crate::app::state::TabId,
    pub input: TextField,
}

/// Live filter of the active panel; Esc restores the mask and cursor.
#[derive(Debug, Clone)]
pub struct QuickFilterPrompt {
    pub input: TextField,
    pub original_mask: Option<String>,
    pub original_cursor: usize,
}

/// Find file (Alt+F7): name and content patterns, options and buttons.
#[derive(Debug, Clone)]
pub struct SearchPrompt {
    pub query: TextField,
    pub content_query: TextField,
    pub search_root: PathBuf,
    pub case_sensitive: bool,
    pub search_target: crate::fs::search::SearchTarget,
    pub cursor_idx: usize,
}

/// Files found by Find file, filled while the search runs.
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<(PathBuf, bool)>,
    pub cursor_idx: usize,
    pub searching: bool,
}
