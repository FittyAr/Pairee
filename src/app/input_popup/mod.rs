pub mod apply_command;
pub mod compress;
pub mod config_dialog;
pub mod color_groups;
pub mod confirm_dialogs;
pub mod context_menu;
pub mod copy;
pub mod copy_filter;
pub mod copy_progress;
pub mod create_link;
pub mod delete;
pub mod describe_file;
pub mod dismiss_only;
pub mod drive_select;
pub mod editor;
pub mod file_attributes;
pub mod file_filter;
pub mod files_highlighting;
pub mod history_list;
pub mod hotlist;
pub mod menu;
pub mod mkdir;
pub mod rename_move;
pub mod save_setup;
pub mod search;
pub mod select_group;
pub mod task_list;
pub mod tree_view;
pub mod user_menu;
pub mod viewer;

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
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::MkDirPrompt { .. } => mkdir::handle(state, key, context),
            PopupType::CopyPrompt { .. } => copy::handle(state, key, context),
            PopupType::ConfirmQuit
            | PopupType::ConfirmInterrupt
            | PopupType::ConfirmOverwrite { .. }
            | PopupType::ConfirmReload { .. }
            | PopupType::ConfirmClearHistory { .. } => confirm_dialogs::handle(state, key, context),
            PopupType::ConfirmDelete { .. } | PopupType::WipeConfirm { .. } => {
                delete::handle(state, key, context)
            }
            PopupType::CopyProgress { .. } => copy_progress::handle(state, key, context),
            PopupType::UserMenu => user_menu::handle(state, key, context),
            PopupType::InternalEditor { .. } | PopupType::EditorSearchPrompt { .. } => {
                editor::handle(state, key, context)
            }
            PopupType::InternalViewer { .. } => viewer::handle(state, key, context),
            PopupType::Menu { .. } => menu::handle(state, key, context),
            PopupType::DriveSelect { .. } => drive_select::handle(state, key, context),
            PopupType::Hotlist { .. } => hotlist::handle(state, key, context),
            PopupType::RenMovPrompt { .. } => rename_move::handle(state, key, context),
            PopupType::SearchPrompt { .. } | PopupType::SearchResults { .. } => {
                search::handle(state, key, context)
            }
            PopupType::TreeView { .. } => tree_view::handle(state, key, context),
            PopupType::ContextMenu { .. } => context_menu::handle(state, key, context),
            PopupType::CompressPrompt { .. } => compress::handle(state, key, context),
        PopupType::CopyMoveFilterPrompt { .. } => copy_filter::handle(state, key, context),
            PopupType::SelectGroupPrompt { .. } => select_group::handle(state, key, context),
            PopupType::ApplyCommandPrompt { .. } => apply_command::handle(state, key, context),
            PopupType::DescribeFilePrompt { .. } => describe_file::handle(state, key, context),
            PopupType::CreateLinkPrompt { .. } => create_link::handle(state, key, context),
            PopupType::FilePanelFilterPrompt { .. } => file_filter::handle(state, key, context),
            PopupType::TaskListDialog { .. } => task_list::handle(state, key, context),
            PopupType::SaveSetupConfirm => save_setup::handle(state, key, context),
            PopupType::ConfigurationDialog { .. } => config_dialog::handle(state, key, context),
            PopupType::ColorGroupsDialog { .. } => color_groups::handle(state, key, context),
            PopupType::FilesHighlightingDialog { .. } => files_highlighting::handle(state, key, context),
            PopupType::FileAttributesDialog { .. } => file_attributes::handle(state, key, context),
            PopupType::CommandHistoryList { .. }
            | PopupType::FileViewHistoryList { .. }
            | PopupType::FoldersHistoryList { .. } => history_list::handle(state, key, context),
            _ => dismiss_only::handle(state, key, context),
        }
    } else {
        Err(())
    }
}
