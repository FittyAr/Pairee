use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelViewMode {
    /// Filename-only, multi-column (Ctrl+1)
    Brief,
    /// Name + basic attributes (Ctrl+2)
    Medium,
    /// Name + size + date (Ctrl+3)
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

impl Default for PanelViewMode {
    fn default() -> Self {
        PanelViewMode::Full
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Name,
    Extension,
    Size,
    Date,
    Unsorted,
}

impl Default for SortField {
    fn default() -> Self {
        SortField::Name
    }
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
pub enum LinkKind {
    Symbolic,
    Hard,
}

#[derive(Debug, Clone)]
pub enum PopupType {
    // ── Basic ────────────────────────────────────────────────────────────────
    Help,
    Error(String),
    /// Neutral informational dialog (not an error).
    Info(String),

    // ── Prompts ──────────────────────────────────────────────────────────────
    MkDirPrompt {
        input: String,
    },
    CopyPrompt {
        input: String,
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    /// Rename/Move prompt — user edits the destination path before committing.
    RenMovPrompt {
        input: String,
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    ConfirmQuit,
    ConfirmInterrupt,
    ConfirmOverwrite {
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        is_move: bool,
        input: Option<String>,
    },
    ConfirmReload {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
        last_search: Option<String>,
    },
    ConfirmClearHistory {
        history_type: String,
    },
    /// Prompt for choosing compression archive name.
    CompressPrompt {
        input: String,
        targets: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    /// Apply command template to selected files.
    ApplyCommandPrompt {
        input: String,
        targets: Vec<PathBuf>,
    },
    /// Add/edit description for a file.
    DescribeFilePrompt {
        path: PathBuf,
        current_desc: String,
        input: String,
    },
    /// Select/unselect files by glob mask.
    SelectGroupPrompt {
        mode: SelectMode,
        query: String,
    },
    /// Create a symlink or hardlink.
    CreateLinkPrompt {
        src: PathBuf,
        dest_input: String,
        kind: LinkKind,
    },
    /// File mask filter for the active panel.
    FilePanelFilterPrompt {
        input: String,
    },

    // ── Confirmations ─────────────────────────────────────────────────────────
    ConfirmDelete {
        paths: Vec<PathBuf>,
    },
    WipeConfirm {
        paths: Vec<PathBuf>,
    },
    SaveSetupConfirm,

    // ── Progress ──────────────────────────────────────────────────────────────
    CopyProgress {
        current_file: String,
        files_copied: usize,
        total_files: usize,
        bytes_copied: u64,
        total_bytes: u64,
    },

    // ── Menus / lists ─────────────────────────────────────────────────────────
    UserMenu,
    Menu {
        active_menu_idx: usize,
        active_item_idx: usize,
    },
    ContextMenu {
        items: Vec<String>,
        cursor_idx: usize,
    },
    DriveSelect {
        panel: ActivePanel,
        drives: Vec<String>,
        cursor_idx: usize,
    },
    Hotlist {
        bookmarks: Vec<(String, PathBuf)>,
        cursor_idx: usize,
    },

    // ── Sort modes ────────────────────────────────────────────────────────────
    SortModesDialog {
        current: SortField,
        reverse: bool,
        cursor_idx: usize,
    },

    // ── Editors / viewers ─────────────────────────────────────────────────────
    InternalEditor {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
        last_search: Option<String>,
    },
    EditorSearchPrompt {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
        last_search: Option<String>,
        query: String,
    },
    InternalViewer {
        viewer: crate::ui::viewer::ViewerState,
    },
    QuickViewPanel {
        path: PathBuf,
        content: Vec<String>,
        scroll: usize,
    },

    // ── File info ─────────────────────────────────────────────────────────────
    InfoPanel {
        lines: Vec<String>,
    },
    FileAttributesDialog {
        attrs: FileAttrsSnapshot,
        mode_input: String,
    },

    // ── Search ────────────────────────────────────────────────────────────────
    SearchPrompt {
        query: String,
        content_query: String,
        search_root: PathBuf,
        focus_content: bool,
    },
    SearchResults {
        query: String,
        results: Vec<PathBuf>,
        cursor_idx: usize,
    },

    // ── History ───────────────────────────────────────────────────────────────
    CommandHistoryList {
        entries: Vec<String>,
        cursor_idx: usize,
    },
    FileViewHistoryList {
        entries: Vec<PathBuf>,
        cursor_idx: usize,
    },
    FoldersHistoryList {
        entries: Vec<PathBuf>,
        cursor_idx: usize,
    },

    // ── Compare ───────────────────────────────────────────────────────────────
    CompareFoldersResult {
        diff: Vec<crate::fs::compare::CompareEntry>,
        cursor_idx: usize,
    },

    // ── OS tools ─────────────────────────────────────────────────────────────
    TaskListDialog {
        tasks: Vec<ProcessEntry>,
        cursor_idx: usize,
    },

    // ── File associations ─────────────────────────────────────────────────────
    FileAssociationsDialog {
        rules: Vec<crate::config::associations::AssocRule>,
        cursor_idx: usize,
    },

    TreeView {
        nodes: Vec<TreeNode>,
        cursor_idx: usize,
        panel: ActivePanel,
    },

    // ── Archive commands ──────────────────────────────────────────────────────
    ArchiveCommandsMenu {
        archive_path: PathBuf,
        items: Vec<String>,
        cursor_idx: usize,
    },

    ConfigurationDialog {
        active_tab: usize,
        cursor_idx: usize,
        editing_value: bool,
        edit_buffer: String,
        settings: crate::config::settings::Settings,
    },
}
