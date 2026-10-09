//! Action name parsing and embedded preset TOML helpers.
//!
//! Chord validation and dispatch live in [`super::loader`] / [`super::resolver`]
//! via the `keybinds` crate — do not reintroduce string-hash key maps here.

use super::actions::Action;

const EMBEDDED_NORTON: &str = include_str!("../../keymaps/norton.toml");
const EMBEDDED_NEOVIM: &str = include_str!("../../keymaps/neovim.toml");
const EMBEDDED_VSCODE: &str = include_str!("../../keymaps/vscode.toml");

/// Returns the shipped TOML body for a built-in preset (for first-run install).
pub fn get_builtin_preset_toml(preset: &str) -> String {
    match preset.to_lowercase().as_str() {
        "neovim" | "vim" => EMBEDDED_NEOVIM.to_string(),
        "vscode" | "modern" => EMBEDDED_VSCODE.to_string(),
        _ => EMBEDDED_NORTON.to_string(),
    }
}

/// Plain (non-parameterised) action names of `keymaps/*.toml`.
const ACTION_NAMES: &[(&str, Action)] = &[
    ("move_up", Action::MoveUp),
    ("move_down", Action::MoveDown),
    ("page_up", Action::PageUp),
    ("page_down", Action::PageDown),
    ("go_to_top", Action::GoToTop),
    ("go_to_bottom", Action::GoToBottom),
    ("change_panel", Action::ChangePanel),
    ("select_item", Action::SelectItem),
    ("execute", Action::Execute),
    ("go_parent", Action::GoParent),
    ("open_archive", Action::OpenArchive),
    ("panel_view_brief", Action::PanelViewBrief),
    ("panel_view_medium", Action::PanelViewMedium),
    ("panel_view_full", Action::PanelViewFull),
    ("panel_view_wide", Action::PanelViewWide),
    ("panel_view_detailed", Action::PanelViewDetailed),
    ("panel_view_descriptions", Action::PanelViewDescriptions),
    ("panel_view_file_owners", Action::PanelViewFileOwners),
    ("panel_view_file_links", Action::PanelViewFileLinks),
    ("panel_view_alt_full", Action::PanelViewAltFull),
    ("toggle_panel_left", Action::TogglePanelLeft),
    ("toggle_panel_right", Action::TogglePanelRight),
    ("toggle_both_panels", Action::ToggleBothPanels),
    ("info_panel", Action::InfoPanel),
    ("quick_view", Action::QuickView),
    ("sort_modes", Action::SortModes),
    ("toggle_long_names", Action::ToggleLongNames),
    ("help", Action::Help),
    ("about", Action::About),
    ("user_menu", Action::UserMenu),
    ("view", Action::View),
    ("view_alt", Action::ViewAlt),
    ("edit", Action::Edit),
    ("copy", Action::Copy),
    ("copy_path", Action::CopyPath),
    ("move", Action::Move),
    ("rename", Action::Rename),
    ("multi_rename_tool", Action::MultiRename),
    ("undo_file_operation", Action::UndoFileOp),
    ("redo_file_operation", Action::RedoFileOp),
    ("mkdir", Action::MkDir),
    ("delete", Action::Delete),
    ("menu", Action::Menu),
    ("quit", Action::Quit),
    ("plugin_menu", Action::PluginMenu),
    ("install_dev_plugin", Action::InstallDevPlugin),
    ("screens_list", Action::ScreensList),
    ("next_screen", Action::NextScreen),
    ("prev_screen", Action::PrevScreen),
    ("print_file", Action::PrintFile),
    ("create_link", Action::CreateLink),
    ("wipe_file", Action::WipeFile),
    ("file_attributes", Action::FileAttributes),
    ("apply_command", Action::ApplyCommand),
    ("describe_file", Action::DescribeFile),
    ("compress_files", Action::CompressFiles),
    ("extract_archive", Action::ExtractArchive),
    ("archive_commands", Action::ArchiveCommands),
    ("select_group", Action::SelectGroup),
    ("unselect_group", Action::UnselectGroup),
    ("invert_selection", Action::InvertSelection),
    ("restore_selection", Action::RestoreSelection),
    ("find_file", Action::FindFile),
    ("command_history", Action::CommandHistory),
    ("file_view_history", Action::FileViewHistory),
    ("folders_history", Action::FoldersHistory),
    ("compare_folder", Action::CompareFolder),
    ("sync_dirs", Action::SyncDirs),
    ("calculate_folder_sizes", Action::CalculateFolderSizes),
    ("disk_usage", Action::DiskUsage),
    ("edit_user_menu", Action::EditUserMenu),
    ("file_associations", Action::FileAssociations),
    ("folder_shortcuts_config", Action::FolderShortcutsConfig),
    ("hotlist", Action::Hotlist),
    ("file_panel_filter", Action::FilePanelFilter),
    ("quick_filter", Action::QuickFilter),
    ("task_list", Action::TaskList),
    ("save_setup", Action::SaveSetup),
    ("system_settings", Action::SystemSettings),
    ("sort_by_name", Action::SortByName),
    ("sort_by_extension", Action::SortByExtension),
    ("sort_by_write_time", Action::SortByWriteTime),
    ("sort_by_size", Action::SortBySize),
    ("sort_unsorted", Action::SortUnsorted),
    ("sort_by_creation_time", Action::SortByCreationTime),
    ("sort_by_access_time", Action::SortByAccessTime),
    ("sort_by_description", Action::SortByDescription),
    ("sort_by_owner", Action::SortByOwner),
    ("toggle_hidden", Action::ToggleHidden),
    ("focus_cli", Action::FocusCli),
    ("unfocus", Action::Unfocus),
    ("refresh", Action::Refresh),
    ("reread_panel", Action::RereadPanel),
    ("swap_panels", Action::SwapPanels),
    ("drive_select_left", Action::DriveSelectLeft),
    ("drive_select_right", Action::DriveSelectRight),
    ("context_menu", Action::ContextMenu),
    ("video_mode", Action::VideoMode),
    ("tree_view", Action::TreeView),
    ("cycle_fkeys_modifiers", Action::CycleFKeysModifiers),
    ("ssh_connect", Action::SshConnect),
    ("ssh_disconnect", Action::SshDisconnect),
    ("open_git_panel", Action::OpenGitPanel),
    ("git_init", Action::GitInit),
    ("git_clone", Action::GitClone),
    ("toggle_sort_reverse", Action::ToggleSortReverse),
    ("check_for_updates", Action::CheckForUpdates),
    ("toggle_transfer_panel", Action::ToggleTransferPanel),
    ("command_palette", Action::CommandPalette),
    ("which_key", Action::WhichKey),
];

/// Converts a snake_case action name string into an `Action` variant.
/// Used when loading `keymaps/*.toml` and `custom_bindings`.
pub fn parse_action_name(name: &str) -> Option<Action> {
    // Handle parameterised variants first
    if let Some(rest) = name.strip_prefix("go_folder_shortcut_") {
        if let Ok(n) = rest.parse::<u8>()
            && (1..=9).contains(&n)
        {
            return Some(Action::GoFolderShortcut(n));
        }
        return None;
    }

    let name_lower = name.to_lowercase();
    let mut clean_name = name_lower.as_str();

    // Strip known suffixes that allow mapping multiple keys to the same action in TOML
    for suffix in &[
        "_arrow", "_pgkey", "_home", "_end", "_enter", "_bs", "_insert", "_fkey", "_alt", "_shift",
        "_rename", "_f10",
    ] {
        if let Some(stripped) = clean_name.strip_suffix(suffix) {
            clean_name = stripped;
            break;
        }
    }

    ACTION_NAMES
        .iter()
        .find(|(action_name, _)| *action_name == clean_name)
        .map(|(_, action)| *action)
        .or_else(|| parse_tab_action(clean_name))
}

/// Folder tab actions (`go_to_tab_1` … `go_to_tab_9` and the tab commands).
fn parse_tab_action(name: &str) -> Option<Action> {
    if let Some(rest) = name.strip_prefix("go_to_tab_") {
        return rest
            .parse::<u8>()
            .ok()
            .filter(|n| (1..=9).contains(n))
            .map(Action::GoToTab);
    }
    match name {
        "new_tab" => Some(Action::NewTab),
        "close_tab" => Some(Action::CloseTab),
        "next_tab" => Some(Action::NextTab),
        "prev_tab" => Some(Action::PrevTab),
        "move_tab_left" => Some(Action::MoveTabLeft),
        "move_tab_right" => Some(Action::MoveTabRight),
        "toggle_tab_lock" => Some(Action::ToggleTabLock),
        "rename_tab" => Some(Action::RenameTab),
        "open_in_new_tab" => Some(Action::OpenInNewTab),
        _ => None,
    }
}
