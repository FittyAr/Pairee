//! Overlay dialogs. Family payloads live beside the enum so PopupType
//! stays a thin tag; the heaviest variant (quick-view image) is boxed.

mod config_dialog;
mod file_ops;
mod git_panel;
mod git_prompts;
mod paste;
mod plugin;
mod plugin_menu;
mod plugin_widget;
mod quickview;
mod ssh;

pub use config_dialog::ConfigurationDialogState;
pub use file_ops::CopyMovePromptState;
pub use git_panel::GitPanelState;
pub use git_prompts::{
    GitBranchCreatePromptState, GitBranchRenamePromptState, GitCommitPromptState,
    GitConfirmActionState, GitConfirmCheckoutState, GitDiffViewState, GitPromptPopup,
    GitStashSavePromptState,
};
pub use plugin::PluginDialog;
pub use plugin_menu::PluginMenuState;
pub use plugin_widget::PluginWidget;
pub use quickview::QuickViewDialog;
pub use ssh::SshConnectPromptState;

use super::types::{
    ActivePanel, AdminOpKind, FileAttrsSnapshot, LinkKind, ProcessEntry, SelectMode, SortField,
    TreeNode, TreeViewCaller,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PopupType {
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
    Info(String),

    MkDirPrompt {
        input: String,
        cursor_idx: usize,
        process_multiple: bool,
    },
    CopyPrompt(CopyMovePromptState),
    MovePrompt(CopyMovePromptState),
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
    CompressPrompt {
        input: String,
        targets: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    ApplyCommandPrompt {
        input: String,
        targets: Vec<PathBuf>,
    },
    DescribeFilePrompt {
        path: PathBuf,
        current_desc: String,
        input: String,
    },
    SelectGroupPrompt {
        mode: SelectMode,
        query: String,
    },
    CreateLinkPrompt {
        src: PathBuf,
        dest_input: String,
        kind: LinkKind,
    },
    FilePanelFilterPrompt {
        input: String,
    },
    QuickFilterPrompt {
        input: String,
        original_mask: Option<String>,
        original_cursor: usize,
    },
    CopyMoveFilterPrompt {
        input: String,
        previous: Box<PopupType>,
    },
    SelectDevPlugin {
        options: Vec<(String, String)>,
        cursor_idx: usize,
        previous_popup: Box<PopupType>,
    },

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
    PluginMenu(PluginMenuState),

    SortModesDialog {
        current: SortField,
        reverse: bool,
        cursor_idx: usize,
    },

    ScreensMenu {
        cursor_idx: usize,
        suspended_popup: Option<Box<PopupType>>,
    },

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
    QuickViewPanel(Box<QuickViewDialog>),

    InfoPanel {
        lines: Vec<String>,
    },
    FileAttributesDialog {
        attrs: FileAttrsSnapshot,
        mode_input: String,
    },

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
        results: Vec<(PathBuf, bool)>,
        cursor_idx: usize,
        searching: bool,
    },

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

    CompareFoldersResult {
        diff: Vec<crate::fs::compare::CompareEntry>,
        cursor_idx: usize,
    },

    TaskListDialog {
        tasks: Vec<ProcessEntry>,
        cursor_idx: usize,
        filter_query: String,
        is_filtering: bool,
    },

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

    ArchiveCommandsMenu {
        archive_path: PathBuf,
        items: Vec<String>,
        cursor_idx: usize,
    },

    ConfigurationDialog(ConfigurationDialogState),

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

    GitPanel(GitPanelState),
    GitPrompt(GitPromptPopup),

    SshConnectPrompt(SshConnectPromptState),

    UpdateAvailable {
        info: crate::update::UpdateInfo,
        cursor_idx: usize,
        install_progress: Option<f32>,
        error: Option<String>,
        scroll_y: usize,
    },

    OnboardingKeymap {
        cursor_idx: usize,
    },

    CommandPalette {
        query: String,
        cursor_idx: usize,
        items: Vec<(String, crate::keybindings::Action)>,
    },

    WhichKey {
        query: String,
        cursor_idx: usize,
        items: Vec<(String, String, crate::keybindings::Action)>,
    },

    Plugin(PluginDialog),
}
