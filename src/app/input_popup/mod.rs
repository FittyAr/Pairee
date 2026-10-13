pub mod about;
pub mod apply_command;
pub mod archive_commands;
pub mod color_groups;
pub mod color_list;
pub mod command_palette;
pub mod compress;
pub mod config_dialog;
pub mod confirm_dialogs;
pub mod connection;
pub mod context_menu;
pub mod copy_filter;
pub mod create_link;
pub mod credential;
pub mod delete;
pub mod describe_file;
pub mod diff_view;
pub mod disk_usage;
pub mod dismiss_only;
pub mod drive_select;
pub mod dupes;
pub mod editor;
pub mod file_associations;
pub mod file_attributes;
pub mod file_compare;
pub mod file_filter;
pub mod files_highlighting;
pub mod folder_jump;
pub mod folder_shortcuts;
pub mod help;
pub mod history_list;
pub mod hotlist;
pub mod list_keys;
pub mod menu;
pub mod mkdir;
pub mod multi_rename;
pub mod onboarding;
pub mod panel_search;
pub mod plugin_dialogs;
pub mod plugin_menu;
pub mod rename;
pub mod rename_tab;
pub mod screens_menu;
pub mod search;
pub mod select_group;
pub mod shortcuts;
pub mod sort_modes;
pub mod sync_dirs;
pub mod task_list;
pub mod transfer_panel;
pub mod transfer_prompt;
pub mod tree_view;
pub mod user_menu;
pub mod viewer;

pub mod git_commit_prompt;
pub mod git_confirm_checkout;
pub mod git_new_popups;
pub mod git_panel;
pub mod git_progress;

pub mod update_popup;
pub mod yazi_popup;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Captures keyboard input for active popups.
pub fn handle_popup_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    // Pick the handler by reference: the popup itself is never cloned here.
    let handler = state.dialogs.top().ok_or(())?.dialog().key_handler();
    handler(state, key, context)
}
