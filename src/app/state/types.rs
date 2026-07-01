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
pub enum TreeViewCaller {
    Panel(ActivePanel),
    CopyPrompt { previous: Box<PopupType> },
    RenMovPrompt { previous: Box<PopupType> },
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
    Delete,
    MkDir,
    RenameMove { dst: PathBuf },
    Copy { dst: PathBuf },
}

#[derive(Debug, Clone)]
pub enum BackgroundOpContext {
    Copy {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
    Move {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub enum PopupType {
    // ── Basic ────────────────────────────────────────────────────────────────
    Help {
        mode: usize,                         // 0 = list focus, 1 = reader focus
        docs: Vec<(String, PathBuf)>,        // Core docs
        plugin_docs: Vec<(String, PathBuf)>, // Plugin docs
        active_tab: usize,                   // 0 = Core Help, 1 = Plugins Help
        cursor_idx: usize,
        scroll_y: usize,
        active_content: Option<String>,
    },
    About {
        scroll_y: usize,
    },
    Error(String),
    /// Neutral informational dialog (not an error).
    Info(String),

    // ── Prompts ──────────────────────────────────────────────────────────────
    MkDirPrompt {
        input: String,
        cursor_idx: usize,
        process_multiple: bool,
    },
    CopyPrompt {
        input: String,
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        cursor_idx: usize,
        already_existing: usize,
        process_multiple: bool,
        copy_access_mode: bool,
        copy_extended_attributes: bool,
        disable_write_cache: bool,
        produce_sparse_files: bool,
        use_copy_on_write: bool,
        symlink_mode: usize,
        use_filter: bool,
        filter_mask: String,
    },
    /// Rename/Move prompt — user edits the destination path before committing.
    RenMovPrompt {
        input: String,
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        cursor_idx: usize,
        already_existing: usize,
        process_multiple: bool,
        copy_access_mode: bool,
        copy_extended_attributes: bool,
        disable_write_cache: bool,
        produce_sparse_files: bool,
        use_copy_on_write: bool,
        symlink_mode: usize,
        use_filter: bool,
        filter_mask: String,
    },
    ConfirmQuit,
    ConfirmInterrupt,
    ConfirmOverwrite {
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        is_move: bool,
        input: Option<String>,
    },
    ConfirmReload,
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
    /// File mask filter for the active panel (glob filter).
    FilePanelFilterPrompt {
        input: String,
    },
    /// Quick filter prompt for the active panel (real-time fragment filter).
    QuickFilterPrompt {
        input: String,
        original_mask: Option<String>,
        original_cursor: usize,
    },
    /// Filter mask input specifically for Copy/Move popups
    CopyMoveFilterPrompt {
        input: String,
        previous: Box<PopupType>,
    },

    // ── Confirmations ─────────────────────────────────────────────────────────
    ConfirmDelete {
        paths: Vec<PathBuf>,
        cursor_idx: usize,
    },
    WipeConfirm {
        paths: Vec<PathBuf>,
    },
    ConfirmRetryAsAdmin {
        paths: Vec<PathBuf>,
        op_kind: AdminOpKind,
    },
    SaveSetupConfirm,

    // ── Progress ──────────────────────────────────────────────────────────────
    CopyProgress {
        is_move: bool,
        current_file: String,
        files_copied: usize,
        total_files: usize,
        bytes_copied: u64,
        total_bytes: u64,
    },

    // ── Menus / lists ─────────────────────────────────────────────────────────
    UserMenu {
        cursor_idx: usize,
    },
    Menu {
        active_menu_idx: usize,
        active_item_idx: Option<usize>,
        active_submenu_idx: Option<usize>,
        active_submenu_item_idx: Option<usize>,
    },
    YaziSortPopup,
    YaziViewPopup,
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
    PluginMenu {
        active_tab: usize,
        cursor_idx: usize,
        installed: Vec<(String, String, bool, bool, Option<String>)>,
        registry: Vec<(String, String, String, String)>,
        search_query: String,
        is_searching: bool,
        editing_query: bool,
        dev_results: String,
        dev_wizard_step: usize,
        dev_wizard_data: Vec<String>,
    },

    // ── Sort modes ────────────────────────────────────────────────────────────
    SortModesDialog {
        current: SortField,
        reverse: bool,
        cursor_idx: usize,
    },

    // ── Screens Menu ──────────────────────────────────────────────────────────
    ScreensMenu {
        cursor_idx: usize,
        suspended_popup: Option<Box<PopupType>>,
    },

    // ── Editors / viewers (Popups for active screens) ─────────────────────────
    EditorSearchPrompt {
        query: String,
        case_sensitive: bool,
        cursor_idx: usize,
    },
    ConfirmDiscardEditorChanges,
    ViewerSearchPrompt {
        query: String,
        case_sensitive: bool,
        cursor_idx: usize,
    },
    QuickViewPanel {
        path: PathBuf,
        content: Vec<String>,
        scroll: usize,
        image_data: Option<image::DynamicImage>,
        plugin_widget: Option<PluginWidget>,
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
        case_sensitive: bool,
        search_target: crate::fs::search::SearchTarget,
        cursor_idx: usize,
    },
    SearchResults {
        query: String,
        results: Vec<(PathBuf, bool)>, // (path, is_dir)
        cursor_idx: usize,
        searching: bool,
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
        filter_query: String,
        is_filtering: bool,
    },

    // ── File associations ─────────────────────────────────────────────────────
    FileAssociationsDialog {
        rules: Vec<crate::config::associations::AssocRule>,
        cursor_idx: usize,
    },

    TreeView {
        nodes: Vec<TreeNode>,
        cursor_idx: usize,
        caller: TreeViewCaller,
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
        focus_on_tabs: bool,
    },

    // ── Colors Configuration ──────────────────────────────────────────────────
    ColorGroupsDialog {
        cursor_idx: usize,
        editing: bool,
        edit_buffer: String,
        theme: crate::config::theme::Theme,
    },
    FilesHighlightingDialog {
        cursor_idx: usize,
        editing: bool,
        edit_buffer: String,
        rules: Vec<crate::ui::highlight::HighlightRule>,
    },

    // ── Git Integration ───────────────────────────────────────────────────────
    /// Main Git panel with tabs: Status / Log / Branches
    GitPanel {
        repo_path: std::path::PathBuf,
        /// 0=Status, 1=Log, 2=Branches
        active_tab: usize,
        cursor_idx: usize,
        scroll: usize,
        status_entries: Vec<crate::git::status::GitFileStatus>,
        log_entries: Vec<crate::git::log::CommitInfo>,
        branch_entries: Vec<crate::git::branches::BranchInfo>,
        current_branch: String,
        #[allow(dead_code)]
        pending_action: Option<GitPendingAction>,
    },
    /// Prompt for typing a git commit message
    GitCommitPrompt {
        input: String,
        cursor_idx: usize,
        repo_path: std::path::PathBuf,
    },
    /// Confirmation dialog before checking out a commit or branch
    GitConfirmCheckout {
        /// Branch name or commit hash
        target: String,
        is_branch: bool,
        repo_path: std::path::PathBuf,
    },

    // ── SSH Connection ────────────────────────────────────────────────────────
    SshConnectPrompt {
        panel: ActivePanel,
        input_name: String,
        input_host: String,
        input_port: String,
        input_user: String,
        input_pass: String,
        input_key_path: String,
        cursor_idx: usize,
        selected_preset_idx: Option<usize>,
    },

    // ── Auto-update ────────────────────────────────────────────────
    /// Shown when a newer version of Pairee is available on GitHub Releases.
    UpdateAvailable {
        info: crate::update::UpdateInfo,
        /// 0 = "Update now", 1 = "Remind me later", 2 = "Ignore this version"
        cursor_idx: usize,
        /// If Some, an install is in progress (holds progress 0.0–1.0).
        install_progress: Option<f32>,
        /// Error message if the install failed.
        error: Option<String>,
        /// Scroll offset for release notes.
        scroll_y: usize,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginWidget {
    Paragraph(String),
    Gauge {
        ratio: f64,
        label: String,
    },
    List(Vec<String>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Span {
        text: String,
        style: String,
    },
    Line(Vec<PluginWidget>),
}
