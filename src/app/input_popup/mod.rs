pub mod about;
pub mod apply_command;
pub mod archive_commands;
pub mod color_groups;
pub mod color_list;
pub mod command_palette;
pub mod compress;
pub mod config_dialog;
pub mod confirm_dialogs;
pub mod context_menu;
pub mod copy_filter;
pub mod create_link;
pub mod delete;
pub mod describe_file;
pub mod disk_usage;
pub mod dismiss_only;
pub mod drive_select;
pub mod editor;
pub mod file_associations;
pub mod file_attributes;
pub mod file_filter;
pub mod files_highlighting;
pub mod folder_shortcuts;
pub mod help;
pub mod history_list;
pub mod hotlist;
pub mod menu;
pub mod mkdir;
pub mod multi_rename;
pub mod onboarding;
pub mod plugin_dialogs;
pub mod plugin_menu;
pub mod rename;
pub mod save_setup;
pub mod screens_menu;
pub mod search;
pub mod select_group;
pub mod sort_modes;
pub mod sync_dirs;
pub mod task_list;
pub mod transfer_panel;
pub mod transfer_prompt;
pub mod tree_view;
pub mod user_menu;
pub mod viewer;
pub mod which_key;

pub mod git_commit_prompt;
pub mod git_confirm_checkout;
pub mod git_new_popups;
pub mod git_panel;
pub mod ssh_connect;
pub mod update_popup;
pub mod yazi_popup;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Captures keyboard input for active popups.
pub fn handle_popup_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    // Pick the handler by reference: the popup itself is never cloned here.
    let handler = state.dialogs.top().map(handler_for).ok_or(())?;
    handler(state, key, context)
}

type PopupHandler = fn(&mut AppState, KeyEvent, &mut AppContext) -> Result<Option<Action>, ()>;

/// Esc in the Git progress popup cancels the running operation.
fn git_progress(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if key.code == crossterm::event::KeyCode::Esc {
        state.cancel_git_op();
    }
    Ok(None)
}

/// Maps the active popup to its key handler.
fn handler_for(popup: &PopupType) -> PopupHandler {
    match popup {
        PopupType::Help { .. } => help::handle,
        PopupType::About { .. } => about::handle,
        PopupType::MkDirPrompt { .. } => mkdir::handle,
        PopupType::TransferPrompt(..) => transfer_prompt::handle,
        PopupType::ConfirmQuit
        | PopupType::ConfirmUndo { .. }
        | PopupType::ConfirmInterrupt
        | PopupType::ConfirmReload
        | PopupType::ConfirmDiscardEditorChanges
        | PopupType::ConfirmClearHistory { .. }
        | PopupType::ConfirmRetryAsAdmin { .. } => confirm_dialogs::handle,
        PopupType::ConfirmDelete { .. } | PopupType::WipeConfirm { .. } => delete::handle,
        PopupType::UserMenu { .. } => user_menu::handle,
        PopupType::EditorSearchPrompt(_)
        | PopupType::EditorSaveAsPrompt { .. }
        | PopupType::EditorConfirmOverwrite { .. } => editor::handle,
        PopupType::ViewerSearchPrompt(_) | PopupType::ViewerEncoding { .. } => viewer::handle,
        PopupType::Menu { .. } => menu::handle,
        PopupType::YaziSortPopup | PopupType::YaziViewPopup => yazi_popup::handle,
        PopupType::ScreensMenu { .. } => screens_menu::handle,
        PopupType::DriveSelect { .. } => drive_select::handle,
        PopupType::Hotlist { .. } => hotlist::handle,
        PopupType::FolderShortcuts { .. } => folder_shortcuts::handle,
        PopupType::RenamePrompt { .. } => rename::handle,
        PopupType::MultiRename(..) => multi_rename::handle,
        PopupType::SearchPrompt { .. } | PopupType::SearchResults { .. } => search::handle,
        PopupType::TreeView { .. } => tree_view::handle,
        PopupType::SyncDirs(_) => sync_dirs::handle,
        PopupType::FolderScanProgress => sync_dirs::handle_progress,
        PopupType::ContextMenu { .. } => context_menu::handle,
        PopupType::CompressPrompt { .. } => compress::handle,
        PopupType::ArchiveCommandsMenu { .. } => archive_commands::handle,
        PopupType::CopyMoveFilterPrompt { .. } => copy_filter::handle,
        PopupType::SelectGroupPrompt { .. } => select_group::handle,
        PopupType::ApplyCommandPrompt { .. } => apply_command::handle,
        PopupType::DescribeFilePrompt { .. } => describe_file::handle,
        PopupType::CreateLinkPrompt { .. } => create_link::handle,
        PopupType::FilePanelFilterPrompt { .. } | PopupType::QuickFilterPrompt { .. } => {
            file_filter::handle
        }
        PopupType::TaskListDialog { .. } => task_list::handle,
        PopupType::DiskUsage => disk_usage::handle,
        PopupType::PluginMenu(..) => plugin_menu::handle,
        PopupType::SelectDevPlugin { .. } => plugin_menu::dev::handle_select_popup,
        PopupType::SaveSetupConfirm => save_setup::handle,
        PopupType::ConfigurationDialog(..) => config_dialog::handle,
        PopupType::ColorGroupsDialog { .. } => color_groups::handle,
        PopupType::FilesHighlightingDialog { .. } => files_highlighting::handle,
        PopupType::FileAttributesDialog { .. } => file_attributes::handle,
        PopupType::CommandHistoryList { .. }
        | PopupType::FileViewHistoryList { .. }
        | PopupType::FoldersHistoryList { .. } => history_list::handle,
        PopupType::SshConnectPrompt(..) => ssh_connect::handle,
        PopupType::GitPanel(..) => git_panel::handle,
        PopupType::GitProgress { .. } => git_progress,
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::CommitPrompt(_)) => {
            git_commit_prompt::handle
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmCheckout(_)) => {
            git_confirm_checkout::handle
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::DiffView(_)) => {
            git_new_popups::handle_diff
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::NamePrompt(_)) => {
            git_new_popups::handle_prompt
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(_)) => {
            git_new_popups::handle_confirm_action
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::RemoteManage(_)) => {
            git_new_popups::handle_remote_manage
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::RemoteAdd(_)) => {
            git_new_popups::handle_remote_add
        }
        PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ClonePrompt(_)) => {
            git_new_popups::handle_clone
        }
        PopupType::SortModesDialog { .. } => sort_modes::handle,
        PopupType::UpdateAvailable { .. } => update_popup::handle,
        PopupType::TransferPanel => transfer_panel::handle,
        PopupType::FileAssociationsDialog { .. } => file_associations::handle,
        PopupType::OnboardingKeymap { .. } => onboarding::handle,
        PopupType::CommandPalette { .. } => command_palette::handle,
        PopupType::WhichKey { .. } => which_key::handle,
        PopupType::Plugin(_) => plugin_dialogs::handle,
        _ => dismiss_only::handle,
    }
}
