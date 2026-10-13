//! Lists and tools: history lists, disk usage, folder comparison and its
//! progress, the task list, file associations, the folder tree, file
//! attributes and the transfer panel focus.

use crate::app::state::types::{FileAttrsSnapshot, ProcessEntry, TreeNode, TreeViewCaller};
use crate::app::text_input::TextField;
use std::path::PathBuf;

/// Command line history (Alt+F8).
#[derive(Debug, Clone)]
pub struct CommandHistoryList {
    pub entries: Vec<String>,
    pub cursor_idx: usize,
}

/// History of viewed and edited files (Alt+F11).
#[derive(Debug, Clone)]
pub struct FileViewHistoryList {
    pub entries: Vec<PathBuf>,
    pub cursor_idx: usize,
}

/// History of visited folders (Alt+F12).
#[derive(Debug, Clone)]
pub struct FoldersHistoryList {
    pub entries: Vec<PathBuf>,
    pub cursor_idx: usize,
}

/// Disk usage view; its data lives in `AppState::disk_usage`.
#[derive(Debug, Clone)]
pub struct DiskUsage;

/// Differences found by Compare folders.
#[derive(Debug, Clone)]
pub struct CompareFoldersResult {
    pub diff: Vec<crate::fs::compare::CompareEntry>,
    pub cursor_idx: usize,
}

/// Recursive folder comparison running (Esc cancels).
#[derive(Debug, Clone)]
pub struct FolderScanProgress;

/// Running processes, with a filter.
#[derive(Debug, Clone)]
pub struct TaskListDialog {
    pub tasks: Vec<ProcessEntry>,
    pub cursor_idx: usize,
    pub filter_query: TextField,
    pub is_filtering: bool,
}

/// Editor of the file associations.
#[derive(Debug, Clone)]
pub struct FileAssociationsDialog {
    pub rules: Vec<crate::config::associations::AssocRule>,
    pub cursor_idx: usize,
    pub editing_idx: Option<usize>,
    /// 0 = mask, 1 = open_cmd, 2 = view_cmd
    pub editing_field: usize,
    pub edit_buffer: TextField,
    pub original_rule: Option<crate::config::associations::AssocRule>,
}

/// Folder tree to jump to, or to pick a Copy / Move destination.
#[derive(Debug, Clone)]
pub struct TreeView {
    pub nodes: Vec<TreeNode>,
    pub cursor_idx: usize,
    pub caller: TreeViewCaller,
}

/// Attributes of the entry under the cursor.
#[derive(Debug, Clone)]
pub struct FileAttributesDialog {
    pub attrs: FileAttrsSnapshot,
    pub mode_input: String,
}

/// The transfer panel has the keys (it is painted by `ui::transfer`).
#[derive(Debug, Clone)]
pub struct TransferPanel;
