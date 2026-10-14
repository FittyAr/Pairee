//! Overlay dialogs. Each dialog is a type implementing [`Dialog`];
//! `PopupType` is the closed set of them, generated from `dialog_list!`.
//! The heaviest ones (quick-view image, multi-rename) are boxed.

mod config_dialog;
mod confirm;
mod connection;
mod credential;
mod dialog;
mod editor_find;
mod editor_viewer;
mod file_ops;
mod filters;
pub mod forms;
mod git_panel;
mod git_prompts;
mod menus;
mod messages;
mod multi_rename;
mod paste;
mod plugin;
mod plugin_menu;
mod plugin_settings;
pub mod plugin_widget;
mod prompts;
mod quickview;
mod settings;
pub mod sync;
mod text_search;
mod tools;
mod views;

pub use config_dialog::{ConfigDraft, ConfigurationDialogState};
pub use confirm::{
    ConfirmClearHistory, ConfirmDiscardEditorChanges, ConfirmInterrupt, ConfirmQuit, ConfirmReload,
    ConfirmRetryAsAdmin, ConfirmUndo, ConfirmUninstallPlugin, SaveSetupConfirm,
};
pub use connection::{ConnectionField, ConnectionPromptState};
pub use credential::CredentialPrompt;
use dialog::define_popup_type;
pub use dialog::{Dialog, DialogKind, DialogScroll, KeyHandler, KeyOutcome};
pub use editor_find::{EditorSearchPrompt, FindButton, FindKey, FindRow};
pub use editor_viewer::{
    EditorConfirmOverwrite, EditorSaveAsPrompt, ViewerEncoding, ViewerSearchPrompt,
};
pub use file_ops::{CopyMovePromptState, TransferEndpoints, TransferPromptOp, split_multiple};
pub use filters::{
    FilePanelFilterPrompt, PanelSearch, QuickFilterPrompt, RenameTabPrompt, SearchPrompt,
    SearchResults,
};
pub use git_panel::GitPanelState;
pub use git_prompts::{
    GitClonePromptState, GitCommitPromptState, GitConfirmActionState, GitConfirmCheckoutState,
    GitDiffViewState, GitNameAction, GitNamePromptState, GitProgress, GitRemoteAddState,
    GitRemoteManageState,
};
pub use menus::{
    ArchiveCommandsMenu, CommandPalette, ContextMenu, DriveSelect, FolderShortcuts, Hotlist, Menu,
    OnboardingKeymap, ScreensMenu, SortModesDialog, UserMenu, UserMenuItem, YaziSortPopup,
    YaziViewPopup,
};
pub use messages::{About, ErrorMessage, Help, InfoMessage, InfoPanel};
pub use multi_rename::{MultiRenameState, RenameField};
pub use plugin::PluginDialog;
pub use plugin_menu::{PluginMenuState, SelectDevPlugin};
pub use plugin_settings::{
    BUTTON_DEFAULTS, BUTTON_SAVE, PluginSettingRow, PluginSettingValue, PluginSettingsDialog,
};
pub use plugin_widget::{PluginPreview, Renderable};
pub use prompts::{
    ApplyCommandPrompt, CompressPrompt, ConfirmDelete, CopyMoveFilterPrompt, CreateLinkPrompt,
    DescribeFilePrompt, MkDirPrompt, RenamePrompt, SelectGroupPrompt, WipeConfirm,
};
pub use quickview::QuickViewDialog;
pub use settings::{ColorGroupsDialog, FilesHighlightingDialog, UpdateAvailable};
pub use sync::{SyncDialog, SyncReview};
pub use text_search::{SearchKey, TextSearchState};
pub use tools::{
    CommandHistoryList, CompareFoldersResult, DiskUsage, FileAssociationsDialog,
    FileAttributesDialog, FileViewHistoryList, FolderScanProgress, FoldersHistoryList,
    TaskListDialog, TransferPanel, TreeView,
};
pub use views::{
    CompareStatus, DiffViewer, DupeOptionsPrompt, DuplicatesView, FileCompareView, FolderJump,
};

/// What the create prompt makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    Folder,
    File,
    /// A folder when the name ends with a path separator, else a file
    /// (yazi / nvim-tree `a`).
    Auto,
}

/// Every overlay dialog, once, as `Variant(DialogType)`: `PopupType` (with
/// its dispatch to [`Dialog`]) and the UI's painting dispatch
/// (`ui::popup::view::render_dialog`) are generated from this list, so a
/// new dialog is its type, its `Dialog` and `DialogView` impls and one line
/// here.
macro_rules! dialog_list {
    ($then:ident) => {
        $then! {
            // Messages and questions
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
            ConfirmUninstallPlugin(ConfirmUninstallPlugin),
            SaveSetupConfirm(SaveSetupConfirm),
            // File operations
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
            // Panel prompts, Find file, editor and viewer
            PanelSearch(PanelSearch),
            FilePanelFilterPrompt(FilePanelFilterPrompt),
            RenameTabPrompt(RenameTabPrompt),
            QuickFilterPrompt(QuickFilterPrompt),
            SearchPrompt(SearchPrompt),
            SearchResults(SearchResults),
            EditorSearchPrompt(Box<EditorSearchPrompt>),
            EditorSaveAsPrompt(EditorSaveAsPrompt),
            EditorConfirmOverwrite(EditorConfirmOverwrite),
            ViewerSearchPrompt(ViewerSearchPrompt),
            ViewerEncoding(ViewerEncoding),
            // Menus and pickers
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
            // Lists and tools
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
            // Views: compare files, folder jump, duplicates
            FileCompareView(FileCompareView),
            FolderJump(FolderJump),
            DupeOptionsPrompt(DupeOptionsPrompt),
            DuplicatesView(DuplicatesView),
            // Settings and update
            ConfigurationDialog(ConfigurationDialogState),
            ColorGroupsDialog(ColorGroupsDialog),
            FilesHighlightingDialog(FilesHighlightingDialog),
            UpdateAvailable(UpdateAvailable),
            // Remote connections
            ConnectionPrompt(ConnectionPromptState),
            CredentialPrompt(CredentialPrompt),
            // Git
            GitPanel(GitPanelState),
            GitProgress(GitProgress),
            GitCommitPrompt(GitCommitPromptState),
            GitConfirmCheckout(GitConfirmCheckoutState),
            GitDiffView(GitDiffViewState),
            GitNamePrompt(GitNamePromptState),
            GitConfirmAction(GitConfirmActionState),
            GitRemoteManage(GitRemoteManageState),
            GitRemoteAdd(GitRemoteAddState),
            GitClonePrompt(GitClonePromptState),
            // Plugins
            PluginMenu(PluginMenuState),
            SelectDevPlugin(SelectDevPlugin),
            PluginSettings(PluginSettingsDialog),
            Plugin(PluginDialog),
        }
    };
}
pub(crate) use dialog_list;

dialog_list!(define_popup_type);
