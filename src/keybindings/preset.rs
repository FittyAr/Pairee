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

/// Converts a snake_case action name string into an `Action` variant.
/// Used when loading `keymaps/*.toml` and `custom_bindings`.
pub fn parse_action_name(name: &str) -> Option<Action> {
    // Handle parameterised variants first
    if let Some(rest) = name.strip_prefix("go_folder_shortcut_") {
        if let Ok(n) = rest.parse::<u8>() {
            if (1..=9).contains(&n) {
                return Some(Action::GoFolderShortcut(n));
            }
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

    match clean_name {
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "page_up" => Some(Action::PageUp),
        "page_down" => Some(Action::PageDown),
        "go_to_top" => Some(Action::GoToTop),
        "go_to_bottom" => Some(Action::GoToBottom),
        "change_panel" => Some(Action::ChangePanel),
        "select_item" => Some(Action::SelectItem),
        "execute" => Some(Action::Execute),
        "go_parent" => Some(Action::GoParent),
        "panel_view_brief" => Some(Action::PanelViewBrief),
        "panel_view_medium" => Some(Action::PanelViewMedium),
        "panel_view_full" => Some(Action::PanelViewFull),
        "panel_view_wide" => Some(Action::PanelViewWide),
        "panel_view_detailed" => Some(Action::PanelViewDetailed),
        "panel_view_descriptions" => Some(Action::PanelViewDescriptions),
        "panel_view_file_owners" => Some(Action::PanelViewFileOwners),
        "panel_view_file_links" => Some(Action::PanelViewFileLinks),
        "panel_view_alt_full" => Some(Action::PanelViewAltFull),
        "toggle_panel_left" => Some(Action::TogglePanelLeft),
        "toggle_panel_right" => Some(Action::TogglePanelRight),
        "toggle_both_panels" => Some(Action::ToggleBothPanels),
        "info_panel" => Some(Action::InfoPanel),
        "quick_view" => Some(Action::QuickView),
        "sort_modes" => Some(Action::SortModes),
        "toggle_long_names" => Some(Action::ToggleLongNames),
        "help" => Some(Action::Help),
        "about" => Some(Action::About),
        "user_menu" => Some(Action::UserMenu),
        "view" => Some(Action::View),
        "view_alt" => Some(Action::ViewAlt),
        "edit" => Some(Action::Edit),
        "copy" => Some(Action::Copy),
        "move" => Some(Action::Move),
        "rename" => Some(Action::Rename),
        "mkdir" => Some(Action::MkDir),
        "delete" => Some(Action::Delete),
        "menu" => Some(Action::Menu),
        "quit" => Some(Action::Quit),
        "plugin_menu" => Some(Action::PluginMenu),
        "install_dev_plugin" => Some(Action::InstallDevPlugin),
        "screens_list" => Some(Action::ScreensList),
        "next_screen" => Some(Action::NextScreen),
        "prev_screen" => Some(Action::PrevScreen),
        "print_file" => Some(Action::PrintFile),
        "create_link" => Some(Action::CreateLink),
        "wipe_file" => Some(Action::WipeFile),
        "file_attributes" => Some(Action::FileAttributes),
        "apply_command" => Some(Action::ApplyCommand),
        "describe_file" => Some(Action::DescribeFile),
        "compress_files" => Some(Action::CompressFiles),
        "extract_archive" => Some(Action::ExtractArchive),
        "archive_commands" => Some(Action::ArchiveCommands),
        "select_group" => Some(Action::SelectGroup),
        "unselect_group" => Some(Action::UnselectGroup),
        "invert_selection" => Some(Action::InvertSelection),
        "restore_selection" => Some(Action::RestoreSelection),
        "find_file" => Some(Action::FindFile),
        "command_history" => Some(Action::CommandHistory),
        "file_view_history" => Some(Action::FileViewHistory),
        "folders_history" => Some(Action::FoldersHistory),
        "compare_folder" => Some(Action::CompareFolder),
        "edit_user_menu" => Some(Action::EditUserMenu),
        "file_associations" => Some(Action::FileAssociations),
        "folder_shortcuts_config" => Some(Action::FolderShortcutsConfig),
        "file_panel_filter" => Some(Action::FilePanelFilter),
        "quick_filter" => Some(Action::QuickFilter),
        "task_list" => Some(Action::TaskList),
        "save_setup" => Some(Action::SaveSetup),
        "system_settings" => Some(Action::SystemSettings),
        "sort_by_name" => Some(Action::SortByName),
        "sort_by_extension" => Some(Action::SortByExtension),
        "sort_by_write_time" => Some(Action::SortByWriteTime),
        "sort_by_size" => Some(Action::SortBySize),
        "sort_unsorted" => Some(Action::SortUnsorted),
        "sort_by_creation_time" => Some(Action::SortByCreationTime),
        "sort_by_access_time" => Some(Action::SortByAccessTime),
        "sort_by_description" => Some(Action::SortByDescription),
        "sort_by_owner" => Some(Action::SortByOwner),
        "toggle_hidden" => Some(Action::ToggleHidden),
        "focus_cli" => Some(Action::FocusCli),
        "unfocus" => Some(Action::Unfocus),
        "refresh" => Some(Action::Refresh),
        "reread_panel" => Some(Action::RereadPanel),
        "swap_panels" => Some(Action::SwapPanels),
        "drive_select_left" => Some(Action::DriveSelectLeft),
        "drive_select_right" => Some(Action::DriveSelectRight),
        "context_menu" => Some(Action::ContextMenu),
        "video_mode" => Some(Action::VideoMode),
        "tree_view" => Some(Action::TreeView),
        "cycle_fkeys_modifiers" => Some(Action::CycleFKeysModifiers),
        "ssh_connect" => Some(Action::SshConnect),
        "ssh_disconnect" => Some(Action::SshDisconnect),
        "open_git_panel" => Some(Action::OpenGitPanel),
        "toggle_sort_reverse" => Some(Action::ToggleSortReverse),
        "check_for_updates" => Some(Action::CheckForUpdates),
        "toggle_transfer_panel" => Some(Action::ToggleTransferPanel),
        "command_palette" => Some(Action::CommandPalette),
        _ => None,
    }
}
