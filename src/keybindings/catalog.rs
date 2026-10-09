//! The action catalogue: every bindable [`Action`] with its keymap id,
//! category and flags. Chords stay in `keymaps/*.toml`; this is **not** a
//! second keymap. Labels come from `lang/*.toml` (`action_<id>`).

use super::actions::Action;

/// Grouping used by the shortcuts modal, which-key and the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Navigation,
    Selection,
    Files,
    Clipboard,
    Archives,
    Search,
    View,
    Sort,
    Panels,
    Tabs,
    Git,
    Remote,
    Tools,
    System,
}

impl Category {
    /// i18n key of the category heading (`category_<name>`).
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Navigation => "category_navigation",
            Self::Selection => "category_selection",
            Self::Files => "category_files",
            Self::Clipboard => "category_clipboard",
            Self::Archives => "category_archives",
            Self::Search => "category_search",
            Self::View => "category_view",
            Self::Sort => "category_sort",
            Self::Panels => "category_panels",
            Self::Tabs => "category_tabs",
            Self::Git => "category_git",
            Self::Remote => "category_remote",
            Self::Tools => "category_tools",
            Self::System => "category_system",
        }
    }
}

/// One catalogued action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDef {
    /// Stable snake_case id used in keymap TOML and config overrides.
    pub id: &'static str,
    pub action: Action,
    pub category: Category,
    /// Listed in the command palette.
    pub in_palette: bool,
    /// Must keep a terminal-robust chord in every preset.
    pub essential: bool,
}

const fn def(id: &'static str, action: Action, category: Category) -> ActionDef {
    ActionDef {
        id,
        action,
        category,
        in_palette: true,
        essential: false,
    }
}

/// An action every preset must reach with a chord any terminal delivers.
const fn essential(id: &'static str, action: Action, category: Category) -> ActionDef {
    ActionDef {
        essential: true,
        ..def(id, action, category)
    }
}

use Category as C;

/// Plain (non-parameterised) actions, grouped by category.
pub const CATALOG: &[ActionDef] = &[
    // Navigation
    essential("move_up", Action::MoveUp, C::Navigation),
    essential("move_down", Action::MoveDown, C::Navigation),
    def("page_up", Action::PageUp, C::Navigation),
    def("page_down", Action::PageDown, C::Navigation),
    def("go_to_top", Action::GoToTop, C::Navigation),
    def("go_to_bottom", Action::GoToBottom, C::Navigation),
    essential("execute", Action::Execute, C::Navigation),
    essential("go_parent", Action::GoParent, C::Navigation),
    def("open_archive", Action::OpenArchive, C::Navigation),
    def("hotlist", Action::Hotlist, C::Navigation),
    def(
        "folder_shortcuts_config",
        Action::FolderShortcutsConfig,
        C::Navigation,
    ),
    def("drive_select_left", Action::DriveSelectLeft, C::Navigation),
    def(
        "drive_select_right",
        Action::DriveSelectRight,
        C::Navigation,
    ),
    def("tree_view", Action::TreeView, C::Navigation),
    def("half_page_up", Action::HalfPageUp, C::Navigation),
    def("half_page_down", Action::HalfPageDown, C::Navigation),
    def("history_back", Action::HistoryBack, C::Navigation),
    def("history_forward", Action::HistoryForward, C::Navigation),
    def("go_home", Action::GoHome, C::Navigation),
    def("go_root", Action::GoRoot, C::Navigation),
    // Selection
    essential("select_item", Action::SelectItem, C::Selection),
    def("select_group", Action::SelectGroup, C::Selection),
    def("unselect_group", Action::UnselectGroup, C::Selection),
    def("invert_selection", Action::InvertSelection, C::Selection),
    def("restore_selection", Action::RestoreSelection, C::Selection),
    def("select_all", Action::SelectAll, C::Selection),
    def("unselect_all", Action::UnselectAll, C::Selection),
    def("visual_mode", Action::VisualMode, C::Selection),
    // Files
    essential("view", Action::View, C::Files),
    def("view_alt", Action::ViewAlt, C::Files),
    essential("edit", Action::Edit, C::Files),
    essential("copy", Action::Copy, C::Files),
    essential("move", Action::Move, C::Files),
    essential("rename", Action::Rename, C::Files),
    def("rename_basename", Action::RenameBasename, C::Files),
    def("multi_rename_tool", Action::MultiRename, C::Files),
    essential("mkdir", Action::MkDir, C::Files),
    def("new_file", Action::NewFile, C::Files),
    def("create", Action::Create, C::Files),
    essential("delete", Action::Delete, C::Files),
    def("trash", Action::Trash, C::Files),
    def("delete_permanent", Action::DeletePermanent, C::Files),
    def("wipe_file", Action::WipeFile, C::Files),
    def("create_link", Action::CreateLink, C::Files),
    def("file_attributes", Action::FileAttributes, C::Files),
    def("describe_file", Action::DescribeFile, C::Files),
    def("print_file", Action::PrintFile, C::Files),
    def("undo_file_operation", Action::UndoFileOp, C::Files),
    def("redo_file_operation", Action::RedoFileOp, C::Files),
    def("compare_folder", Action::CompareFolder, C::Files),
    def("sync_dirs", Action::SyncDirs, C::Files),
    def(
        "calculate_folder_sizes",
        Action::CalculateFolderSizes,
        C::Files,
    ),
    def("disk_usage", Action::DiskUsage, C::Files),
    def("file_associations", Action::FileAssociations, C::Files),
    def("apply_command", Action::ApplyCommand, C::Files),
    // Clipboard
    def("yank", Action::Yank, C::Clipboard),
    def("cut", Action::Cut, C::Clipboard),
    def("paste", Action::Paste, C::Clipboard),
    def("paste_overwrite", Action::PasteOverwrite, C::Clipboard),
    def("paste_as_link", Action::PasteAsLink, C::Clipboard),
    def("clear_clipboard", Action::ClearClipboard, C::Clipboard),
    def("copy_path", Action::CopyPath, C::Clipboard),
    def("copy_name", Action::CopyName, C::Clipboard),
    def("copy_name_no_ext", Action::CopyNameNoExt, C::Clipboard),
    def("copy_dir_path", Action::CopyDirPath, C::Clipboard),
    // Archives
    def("compress_files", Action::CompressFiles, C::Archives),
    def("extract_archive", Action::ExtractArchive, C::Archives),
    def("archive_commands", Action::ArchiveCommands, C::Archives),
    // Search & history
    def("find_file", Action::FindFile, C::Search),
    def("find_in_panel", Action::FindInPanel, C::Search),
    def("find_next", Action::FindNext, C::Search),
    def("find_prev", Action::FindPrev, C::Search),
    def("quick_filter", Action::QuickFilter, C::Search),
    def("file_panel_filter", Action::FilePanelFilter, C::Search),
    def("command_history", Action::CommandHistory, C::Search),
    def("file_view_history", Action::FileViewHistory, C::Search),
    def("folders_history", Action::FoldersHistory, C::Search),
    // View
    def("panel_view_brief", Action::PanelViewBrief, C::View),
    def("panel_view_medium", Action::PanelViewMedium, C::View),
    def("panel_view_full", Action::PanelViewFull, C::View),
    def("panel_view_wide", Action::PanelViewWide, C::View),
    def("panel_view_detailed", Action::PanelViewDetailed, C::View),
    def(
        "panel_view_descriptions",
        Action::PanelViewDescriptions,
        C::View,
    ),
    def(
        "panel_view_file_owners",
        Action::PanelViewFileOwners,
        C::View,
    ),
    def("panel_view_file_links", Action::PanelViewFileLinks, C::View),
    def("panel_view_alt_full", Action::PanelViewAltFull, C::View),
    def("toggle_hidden", Action::ToggleHidden, C::View),
    def("toggle_long_names", Action::ToggleLongNames, C::View),
    def("refresh", Action::Refresh, C::View),
    def("reread_panel", Action::RereadPanel, C::View),
    def("video_mode", Action::VideoMode, C::View),
    def(
        "cycle_fkeys_modifiers",
        Action::CycleFKeysModifiers,
        C::View,
    ),
    def("view_mode_menu", Action::ViewModeMenu, C::View),
    def("cycle_panel_view", Action::CyclePanelView, C::View),
    // Sort
    def("sort_modes", Action::SortModes, C::Sort),
    def("sort_by_name", Action::SortByName, C::Sort),
    def("sort_by_extension", Action::SortByExtension, C::Sort),
    def("sort_by_write_time", Action::SortByWriteTime, C::Sort),
    def("sort_by_size", Action::SortBySize, C::Sort),
    def("sort_unsorted", Action::SortUnsorted, C::Sort),
    def("sort_by_creation_time", Action::SortByCreationTime, C::Sort),
    def("sort_by_access_time", Action::SortByAccessTime, C::Sort),
    def("sort_by_description", Action::SortByDescription, C::Sort),
    def("sort_by_owner", Action::SortByOwner, C::Sort),
    def("toggle_sort_reverse", Action::ToggleSortReverse, C::Sort),
    def("sort_menu", Action::SortMenu, C::Sort),
    def("cycle_sort", Action::CycleSort, C::Sort),
    // Panels
    essential("change_panel", Action::ChangePanel, C::Panels),
    def("swap_panels", Action::SwapPanels, C::Panels),
    def("toggle_panel_left", Action::TogglePanelLeft, C::Panels),
    def("toggle_panel_right", Action::TogglePanelRight, C::Panels),
    def("toggle_both_panels", Action::ToggleBothPanels, C::Panels),
    def("info_panel", Action::InfoPanel, C::Panels),
    def("quick_view", Action::QuickView, C::Panels),
    def(
        "toggle_transfer_panel",
        Action::ToggleTransferPanel,
        C::Panels,
    ),
    def("focus_left_panel", Action::FocusLeftPanel, C::Panels),
    def("focus_right_panel", Action::FocusRightPanel, C::Panels),
    // Tabs & screens
    def("new_tab", Action::NewTab, C::Tabs),
    def("close_tab", Action::CloseTab, C::Tabs),
    def("next_tab", Action::NextTab, C::Tabs),
    def("prev_tab", Action::PrevTab, C::Tabs),
    def("move_tab_left", Action::MoveTabLeft, C::Tabs),
    def("move_tab_right", Action::MoveTabRight, C::Tabs),
    def("toggle_tab_lock", Action::ToggleTabLock, C::Tabs),
    def("rename_tab", Action::RenameTab, C::Tabs),
    def("open_in_new_tab", Action::OpenInNewTab, C::Tabs),
    def("screens_list", Action::ScreensList, C::Tabs),
    def("next_screen", Action::NextScreen, C::Tabs),
    def("prev_screen", Action::PrevScreen, C::Tabs),
    // Git
    def("open_git_panel", Action::OpenGitPanel, C::Git),
    def("git_init", Action::GitInit, C::Git),
    def("git_clone", Action::GitClone, C::Git),
    // Remote
    def("ssh_connect", Action::SshConnect, C::Remote),
    def("ssh_disconnect", Action::SshDisconnect, C::Remote),
    // Tools
    def("user_menu", Action::UserMenu, C::Tools),
    def("edit_user_menu", Action::EditUserMenu, C::Tools),
    def("task_list", Action::TaskList, C::Tools),
    def("plugin_menu", Action::PluginMenu, C::Tools),
    def("install_dev_plugin", Action::InstallDevPlugin, C::Tools),
    def("focus_cli", Action::FocusCli, C::Tools),
    // System
    essential("help", Action::Help, C::System),
    def("about", Action::About, C::System),
    essential("menu", Action::Menu, C::System),
    def("context_menu", Action::ContextMenu, C::System),
    essential("command_palette", Action::CommandPalette, C::System),
    essential("which_key", Action::WhichKey, C::System),
    essential("unfocus", Action::Unfocus, C::System),
    def("save_setup", Action::SaveSetup, C::System),
    def("system_settings", Action::SystemSettings, C::System),
    def("check_for_updates", Action::CheckForUpdates, C::System),
    essential("quit", Action::Quit, C::System),
];

/// Nine catalogue rows `<prefix>1` … `<prefix>9` for a numbered action.
macro_rules! numbered {
    ($prefix:literal, $variant:ident, $category:expr) => {
        [
            numbered!(@one $prefix, $variant, $category, 1),
            numbered!(@one $prefix, $variant, $category, 2),
            numbered!(@one $prefix, $variant, $category, 3),
            numbered!(@one $prefix, $variant, $category, 4),
            numbered!(@one $prefix, $variant, $category, 5),
            numbered!(@one $prefix, $variant, $category, 6),
            numbered!(@one $prefix, $variant, $category, 7),
            numbered!(@one $prefix, $variant, $category, 8),
            numbered!(@one $prefix, $variant, $category, 9),
        ]
    };
    (@one $prefix:literal, $variant:ident, $category:expr, $n:literal) => {
        ActionDef {
            in_palette: false,
            ..def(concat!($prefix, $n), Action::$variant($n), $category)
        }
    };
}

/// `go_folder_shortcut_1` … `_9`.
pub const FOLDER_SHORTCUTS: [ActionDef; 9] =
    numbered!("go_folder_shortcut_", GoFolderShortcut, C::Navigation);
/// `go_to_tab_1` … `_9`.
pub const TAB_SLOTS: [ActionDef; 9] = numbered!("go_to_tab_", GoToTab, C::Tabs);

/// Every catalogued action, plain ones first.
pub fn all_defs() -> impl Iterator<Item = &'static ActionDef> {
    CATALOG
        .iter()
        .chain(FOLDER_SHORTCUTS.iter())
        .chain(TAB_SLOTS.iter())
}
