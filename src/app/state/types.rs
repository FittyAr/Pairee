use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PanelViewMode {
    /// Filename-only, multi-column (Ctrl+1)
    Brief,
    /// Name + basic attributes (Ctrl+2)
    Medium,
    /// Name + size + date (Ctrl+3)
    #[default]
    Full,
    /// Wide single column (Ctrl+4)
    Wide,
    /// Name + permissions + owner + real size (Ctrl+5)
    Detailed,
    /// Name + descript.ion entry (Ctrl+6)
    Descriptions,
    /// Name + owner/group (Ctrl+7)
    FileOwners,
    /// Name + hardlink count (Ctrl+8)
    FileLinks,
    /// User-configurable columns (Ctrl+9)
    AltFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortField {
    #[default]
    Name,
    Extension,
    Size,
    Date,
    Unsorted,
}

// File attribute snapshot (cross-platform subset)
#[derive(Debug, Clone)]
pub struct FileAttrsSnapshot {
    pub path: PathBuf,
    pub readonly: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
    pub owner: String,
    pub nlinks: u64,
}

// OS Process entry (for TaskList popup)
#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub memory_kb: u64,
}

// Tree view node
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub depth: usize,
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub enum SelectMode {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub enum TreeViewCaller {
    Panel(ActivePanel),
    CopyPrompt { previous: Box<PopupType> },
    MovePrompt { previous: Box<PopupType> },
}

#[derive(Debug, Clone)]
pub enum LinkKind {
    Symbolic,
    Hard,
}

/// Pending action queued from within the GitPanel popup.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum GitPendingAction {
    CommitAll,
    Checkout(String),
}

/// Action approved in GitConfirmAction popup.
#[derive(Debug, Clone)]
pub enum GitConfirmedAction {
    DeleteBranch(String),
    MergeBranch(String),
    StashDrop(usize),
    StashPop(usize),
    ResetCommit(String, crate::git::reset::ResetMode),
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub is_dirty: bool,
    pub last_search: Option<String>,
    pub last_case_sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub command: String,
    pub output_lines: Vec<String>,
    pub is_running: bool,
    #[allow(dead_code)]
    pub pid: Option<u32>,
}

/// Progress message emitted by a long-running Developer Tools operation
/// (init, lint, package, install, submit) running in the background.
///
/// The UI drains these from `AppState::dev_progress_rx` on every frame and
/// reflects them as a status line and (when available) a determinate
/// progress bar in the Dev Tools console.
#[derive(Debug, Clone)]
pub struct DevProgress {
    /// Human-readable status (already localized by the caller).
    pub status: String,
    /// Current step / file index (for determinate progress).
    pub current: Option<usize>,
    /// Total step / file count (for determinate progress).
    pub total: Option<usize>,
    /// `true` when the operation has finished (success or failure).
    pub done: bool,
    /// Final result text to dump into the dev console on completion.
    pub result: Option<String>,
    /// Error message on failure; the operation has also finished.
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TerminalUpdate {
    pub screen_idx: usize,
    pub line: Option<String>, // Some(line) = new output, None = process exited
}

#[derive(Debug, Clone)]
pub enum Screen {
    Panels,
    Viewer(crate::ui::viewer::ViewerState),
    Editor(EditorState),
    Terminal(TerminalState),
}

#[derive(Debug, Clone)]
pub enum AdminOpKind {
    MkDir,
    Rename {
        src: std::path::PathBuf,
        target: std::path::PathBuf,
    },
}

pub use super::popup::{PluginWidget, PopupType};
