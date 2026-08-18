//! Overlay dialogs. Family payloads live beside the enum so PopupType
//! stays a thin tag; the heaviest variant (quick-view image) is boxed.

use super::types::{
    ActivePanel, AdminOpKind, FileAttrsSnapshot, GitConfirmedAction, GitPendingAction, LinkKind,
    ProcessEntry, SelectMode, SortField, TreeNode, TreeViewCaller,
};
use std::path::PathBuf;

/// Quick-view overlay (passive panel). Boxed in [PopupType] because of DynamicImage.
#[derive(Debug, Clone)]
pub struct QuickViewDialog {
    pub path: PathBuf,
    pub content: Vec<String>,
    pub scroll: usize,
    pub image_data: Option<image::DynamicImage>,
    pub plugin_widget: Option<PluginWidget>,
}

// Phase C.2: large payloads live in family structs (see QuickViewDialog).
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
    /// Move prompt — user edits the destination path before committing.
    MovePrompt {
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
    /// Rename prompt — user edits only the filename before committing.
    RenamePrompt {
        input: String,
        original: String,
        src_path: PathBuf,
        parent_dir: PathBuf,
        cursor_idx: usize,
    },
    ConfirmQuit,
    ConfirmInterrupt,
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
    SelectDevPlugin {
        options: Vec<(String, String)>,
        cursor_idx: usize,
        previous_popup: Box<PopupType>,
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

    TransferPanel,

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
        /// Full list of plugins from the registry index (unfiltered). Used as
        /// the source for live-filtering in the Search tab.
        all_registry: Vec<(String, String, String, String)>,
        /// Currently visible list after applying `search_query` filter.
        registry: Vec<(String, String, String, String)>,
        search_query: String,
        is_searching: bool,
        editing_query: bool,
        dev_results: String,
        dev_wizard_step: usize,
        dev_wizard_data: Vec<String>,
        /// True while the initial installed-plugins list (and registry index) is
        /// being fetched in the background after opening the menu.
        installed_loading: bool,
        /// Human-readable status text shown while `installed_loading` is true.
        installed_loading_status: String,
        /// True while a Developer Tools operation is running asynchronously.
        dev_loading: bool,
        /// Human-readable status text shown while `dev_loading` is true
        /// (e.g. "Cloning registry…", "Copying files…").
        dev_loading_status: String,
        /// Optional determinate progress `(current, total)` for the running
        /// dev operation. When `None`, the renderer falls back to an
        /// indeterminate (pulsing) indicator.
        dev_loading_progress: Option<(usize, usize)>,
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
    /// Boxed: DynamicImage would otherwise dominate PopupType size.
    QuickViewPanel(Box<QuickViewDialog>),

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
        editing_idx: Option<usize>,
        editing_field: usize, // 0 = mask, 1 = open_cmd, 2 = view_cmd
        edit_buffer: String,
        original_rule: Option<crate::config::associations::AssocRule>,
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
        settings: Box<crate::config::settings::Settings>,
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
    /// Main Git panel with tabs: Status / Log / Branches / Stash
    GitPanel {
        repo_path: std::path::PathBuf,
        /// 0=Status, 1=Log, 2=Branches, 3=Stash
        active_tab: usize,
        cursor_idx: usize,
        scroll: usize,
        status_entries: Vec<crate::git::status::GitFileStatus>,
        log_entries: Vec<crate::git::log::CommitInfo>,
        branch_entries: Vec<crate::git::branches::BranchInfo>,
        stash_entries: Vec<crate::git::stash::StashInfo>,
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
    /// View Git unified diff for a file or commit
    GitDiffView {
        repo_path: std::path::PathBuf,
        file_path: Option<String>,
        commit_hash: Option<String>,
        diff_content: String,
        scroll_y: usize,
        previous_popup: Box<PopupType>,
    },
    /// Prompt for entering new branch name
    GitBranchCreatePrompt {
        input: String,
        cursor_idx: usize,
        repo_path: std::path::PathBuf,
        previous_popup: Box<PopupType>,
    },
    /// Prompt for entering a new name for an existing branch
    GitBranchRenamePrompt {
        input: String,
        cursor_idx: usize,
        old_name: String,
        repo_path: std::path::PathBuf,
        previous_popup: Box<PopupType>,
    },
    /// Prompt for entering a stash message
    GitStashSavePrompt {
        input: String,
        cursor_idx: usize,
        repo_path: std::path::PathBuf,
        previous_popup: Box<PopupType>,
    },
    /// Generic confirmation dialog for Git destructive or integration actions
    GitConfirmAction {
        message: String,
        repo_path: std::path::PathBuf,
        action: GitConfirmedAction,
        previous_popup: Box<PopupType>,
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

    /// Fuzzy command palette (Ctrl+Shift+P): filter and run logical `Action`s.
    CommandPalette {
        query: String,
        cursor_idx: usize,
        /// Display label + action pairs currently matching `query`.
        items: Vec<(String, crate::keybindings::Action)>,
    },

    // ── Plugin dialogs (`pairee.confirm` / `input` / `which`) ───────────────
    /// Reply oneshot lives in [`super::PluginHostState::pending_dialog`].
    PluginConfirm {
        title: String,
        msg: String,
        /// 0 = Yes, 1 = No
        cursor_idx: usize,
        position: Option<crate::plugin::manager::DialogPosition>,
    },
    PluginInput {
        title: String,
        input: String,
        obscure: bool,
        position: Option<crate::plugin::manager::DialogPosition>,
    },
    PluginWhich {
        candidates: Vec<crate::plugin::manager::WhichCandidate>,
        silent: bool,
        position: Option<crate::plugin::manager::DialogPosition>,
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
