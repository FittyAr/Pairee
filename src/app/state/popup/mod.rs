//! Overlay dialogs. Family payloads live beside the enum so PopupType
//! stays a thin tag; the heaviest variant (quick-view image) is boxed.

mod config_dialog;
mod file_ops;
pub mod forms;
mod git_panel;
mod git_prompts;
mod paste;
mod plugin;
mod plugin_menu;
mod plugin_widget;
mod quickview;
mod ssh;
mod text_search;

pub use config_dialog::ConfigurationDialogState;
pub use file_ops::{CopyMovePromptState, TransferPromptOp};
pub use git_panel::GitPanelState;
pub use git_prompts::{
    GitClonePromptState, GitCommitPromptState, GitConfirmActionState, GitConfirmCheckoutState,
    GitDiffViewState, GitNameAction, GitNamePromptState, GitPromptPopup, GitRemoteAddState,
    GitRemoteManageState,
};
pub use plugin::PluginDialog;
pub use plugin_menu::PluginMenuState;
pub use plugin_widget::PluginWidget;
pub use quickview::QuickViewDialog;
pub use ssh::{SshConnectPromptState, SshField};
pub use text_search::{SearchKey, TextSearchState};

use super::types::{
    ActivePanel, AdminOpKind, FileAttrsSnapshot, LinkKind, ProcessEntry, SelectMode, SortField,
    TreeNode, TreeViewCaller,
};
use crate::app::text_input::TextField;
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
        input: TextField,
        cursor_idx: usize,
        process_multiple: bool,
    },
    /// Copy (F5) / Move (F6) dialog.
    TransferPrompt(CopyMovePromptState),
    RenamePrompt {
        input: TextField,
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
        input: TextField,
        targets: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    ApplyCommandPrompt {
        input: TextField,
        targets: Vec<PathBuf>,
    },
    DescribeFilePrompt {
        path: PathBuf,
        current_desc: String,
        input: TextField,
    },
    SelectGroupPrompt {
        mode: SelectMode,
        query: TextField,
    },
    CreateLinkPrompt {
        src: PathBuf,
        dest_input: TextField,
        kind: LinkKind,
    },
    FilePanelFilterPrompt {
        input: TextField,
    },
    QuickFilterPrompt {
        input: TextField,
        original_mask: Option<String>,
        original_cursor: usize,
    },
    CopyMoveFilterPrompt {
        input: TextField,
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
        entries: Vec<crate::config::bookmarks::HotlistEntry>,
        cursor_idx: usize,
    },
    /// Folder shortcut slots 1–9 (contents live in `AppState::folder_shortcuts`).
    FolderShortcuts {
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

    EditorSearchPrompt(TextSearchState),
    ConfirmDiscardEditorChanges,
    /// "Save as" path entry for the built-in editor (Shift+F2).
    EditorSaveAsPrompt {
        input: TextField,
    },
    /// Saving would overwrite a file changed on disk or an existing target.
    EditorConfirmOverwrite {
        target: PathBuf,
        reason: crate::app::editor::open::OverwriteReason,
    },
    ViewerSearchPrompt(TextSearchState),
    QuickViewPanel(Box<QuickViewDialog>),

    InfoPanel {
        lines: Vec<String>,
    },
    FileAttributesDialog {
        attrs: FileAttrsSnapshot,
        mode_input: String,
    },

    SearchPrompt {
        query: TextField,
        content_query: TextField,
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
        filter_query: TextField,
        is_filtering: bool,
    },

    FileAssociationsDialog {
        rules: Vec<crate::config::associations::AssocRule>,
        cursor_idx: usize,
        editing_idx: Option<usize>,
        editing_field: usize, // 0 = mask, 1 = open_cmd, 2 = view_cmd
        edit_buffer: TextField,
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
        /// The color being typed (Enter), if any.
        edit: Option<TextField>,
        theme: crate::config::theme::Theme,
    },
    FilesHighlightingDialog {
        cursor_idx: usize,
        /// The color being typed (Enter), if any.
        edit: Option<TextField>,
        rules: Vec<crate::ui::highlight::HighlightRule>,
    },

    GitPanel(GitPanelState),
    GitPrompt(GitPromptPopup),
    /// Progress of the background Git network operation (`AppState::git_op`).
    GitProgress {
        title: String,
    },

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
        query: TextField,
        cursor_idx: usize,
        items: Vec<(String, crate::keybindings::Action)>,
    },

    WhichKey {
        query: TextField,
        cursor_idx: usize,
        items: Vec<(String, String, crate::keybindings::Action)>,
    },

    Plugin(PluginDialog),
}
