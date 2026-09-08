//! History lists (commands, file views, and visited folders) input dispatcher.

mod handlers;
mod nav;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.dialogs.top().cloned();
    if let Some(p) = popup {
        match p {
            PopupType::CommandHistoryList {
                entries,
                cursor_idx,
            } => handlers::handle_command_history(state, key, context, entries, cursor_idx),
            PopupType::FileViewHistoryList {
                entries,
                cursor_idx,
            } => handlers::handle_file_view_history(state, key, context, entries, cursor_idx),
            PopupType::FoldersHistoryList {
                entries,
                cursor_idx,
            } => handlers::handle_folders_history(state, key, context, entries, cursor_idx),
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
