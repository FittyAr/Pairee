//! Overlay dialogs. Each dialog is a type implementing [`Dialog`];
//! `PopupType` is the closed set of them, generated from `dialog_list!`.
//! The heaviest ones (quick-view image, multi-rename) are boxed.

mod config_dialog;
mod confirm;
mod dialog;
mod editor_viewer;
mod file_ops;
mod filters;
pub mod forms;
mod git_panel;
mod git_prompts;
mod menus;
mod messages;
mod mouse;
mod multi_rename;
mod paste;
mod plugin;
mod plugin_menu;
mod plugin_widget;
mod prompts;
mod quickview;
mod ssh;
pub mod sync;
mod text_search;
mod tools;

pub use config_dialog::{ConfigDraft, ConfigurationDialogState};
pub use confirm::{
    ConfirmClearHistory, ConfirmDiscardEditorChanges, ConfirmInterrupt, ConfirmQuit, ConfirmReload,
    ConfirmRetryAsAdmin, ConfirmUndo, SaveSetupConfirm,
};
use dialog::define_popup_type;
pub use dialog::{Dialog, DialogKind, DialogScroll, KeyHandler, KeyOutcome};
pub use editor_viewer::{
    EditorConfirmOverwrite, EditorSaveAsPrompt, EditorSearchPrompt, ViewerEncoding,
    ViewerSearchPrompt,
};
pub use file_ops::{CopyMovePromptState, TransferEndpoints, TransferPromptOp, split_multiple};
pub use filters::{
    FilePanelFilterPrompt, PanelSearch, QuickFilterPrompt, RenameTabPrompt, SearchPrompt,
    SearchResults,
};
pub use git_panel::GitPanelState;
pub use git_prompts::{
    GitClonePromptState, GitCommitPromptState, GitConfirmActionState, GitConfirmCheckoutState,
    GitDiffViewState, GitNameAction, GitNamePromptState, GitPromptPopup, GitRemoteAddState,
    GitRemoteManageState,
};
pub use menus::{
    ArchiveCommandsMenu, CommandPalette, ContextMenu, DriveSelect, FolderShortcuts, Hotlist, Menu,
    OnboardingKeymap, ScreensMenu, SortModesDialog, UserMenu, YaziSortPopup, YaziViewPopup,
};
pub use messages::{About, ErrorMessage, Help, InfoMessage, InfoPanel};
pub use multi_rename::{MultiRenameState, RenameField};
pub use plugin::PluginDialog;
pub use plugin_menu::PluginMenuState;
pub use plugin_widget::PluginWidget;
pub use prompts::{
    ApplyCommandPrompt, CompressPrompt, ConfirmDelete, CopyMoveFilterPrompt, CreateLinkPrompt,
    DescribeFilePrompt, MkDirPrompt, RenamePrompt, SelectGroupPrompt, WipeConfirm,
};
pub use quickview::QuickViewDialog;
pub use ssh::{SshConnectPromptState, SshField};
pub use sync::{SyncDialog, SyncReview};
pub use text_search::{SearchKey, TextSearchState};
pub use tools::{
    CommandHistoryList, CompareFoldersResult, DiskUsage, FileAssociationsDialog,
    FileAttributesDialog, FileViewHistoryList, FolderScanProgress, FoldersHistoryList,
    TaskListDialog, TransferPanel, TreeView,
};

use crate::app::text_input::TextField;

/// What the create prompt makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    Folder,
    File,
    /// A folder when the name ends with a path separator, else a file
    /// (yazi / nvim-tree `a`).
    Auto,
}

/// Every overlay dialog, once: `PopupType` (with its dispatch to
/// [`Dialog`]) and the UI's painting dispatch are generated from this list.
/// `typed` entries are `Variant(DialogType)`; `legacy` holds the variants
/// not yet moved to their own type.
macro_rules! dialog_list {
    ($then:ident) => {
        $then! {
            typed: [
                Help(Help),
                About(About),
                Error(ErrorMessage),
                Info(InfoMessage),
                InfoPanel(InfoPanel),
                ConfirmQuit(ConfirmQuit),
                ConfirmUndo(ConfirmUndo),
                ConfirmInterrupt(ConfirmInterrupt),
                ConfirmReload(ConfirmReload),
                ConfirmDiscardEditorChanges(ConfirmDiscardEditorChanges),
                ConfirmClearHistory(ConfirmClearHistory),
                ConfirmRetryAsAdmin(ConfirmRetryAsAdmin),
                SaveSetupConfirm(SaveSetupConfirm),
                MkDirPrompt(MkDirPrompt),
                TransferPrompt(CopyMovePromptState),
                RenamePrompt(RenamePrompt),
                MultiRename(Box<MultiRenameState>),
                CompressPrompt(CompressPrompt),
                ApplyCommandPrompt(ApplyCommandPrompt),
                DescribeFilePrompt(DescribeFilePrompt),
                SelectGroupPrompt(SelectGroupPrompt),
                CreateLinkPrompt(CreateLinkPrompt),
                CopyMoveFilterPrompt(CopyMoveFilterPrompt),
                ConfirmDelete(ConfirmDelete),
                WipeConfirm(WipeConfirm),
                PanelSearch(PanelSearch),
                FilePanelFilterPrompt(FilePanelFilterPrompt),
                RenameTabPrompt(RenameTabPrompt),
                QuickFilterPrompt(QuickFilterPrompt),
                SearchPrompt(SearchPrompt),
                SearchResults(SearchResults),
                EditorSaveAsPrompt(EditorSaveAsPrompt),
                EditorConfirmOverwrite(EditorConfirmOverwrite),
                ViewerEncoding(ViewerEncoding),
                EditorSearchPrompt(EditorSearchPrompt),
                ViewerSearchPrompt(ViewerSearchPrompt),
                UserMenu(UserMenu),
                Menu(Menu),
                YaziSortPopup(YaziSortPopup),
                YaziViewPopup(YaziViewPopup),
                ContextMenu(ContextMenu),
                DriveSelect(DriveSelect),
                Hotlist(Hotlist),
                FolderShortcuts(FolderShortcuts),
                SortModesDialog(SortModesDialog),
                ScreensMenu(ScreensMenu),
                ArchiveCommandsMenu(ArchiveCommandsMenu),
                CommandPalette(CommandPalette),
                OnboardingKeymap(OnboardingKeymap),
                Shortcuts(Box<crate::app::shortcuts::state::ShortcutsState>),
                CommandHistoryList(CommandHistoryList),
                FileViewHistoryList(FileViewHistoryList),
                FoldersHistoryList(FoldersHistoryList),
                DiskUsage(DiskUsage),
                CompareFoldersResult(CompareFoldersResult),
                FolderScanProgress(FolderScanProgress),
                SyncDirs(Box<SyncDialog>),
                TaskListDialog(TaskListDialog),
                FileAssociationsDialog(FileAssociationsDialog),
                TreeView(TreeView),
                FileAttributesDialog(FileAttributesDialog),
                TransferPanel(TransferPanel),
                QuickViewPanel(Box<QuickViewDialog>),
            ]
            legacy: [
        SelectDevPlugin {
            options: Vec<(String, String)>,
            cursor_idx: usize,
            previous_popup: Box<PopupType>,
        },
        PluginMenu(PluginMenuState),
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
        Plugin(PluginDialog),
            ]
        }
    };
}
pub(crate) use dialog_list;

dialog_list!(define_popup_type);
