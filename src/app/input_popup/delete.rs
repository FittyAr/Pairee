//! Delete (F8) and wipe confirmations.

use crate::app::actions::fs_ops::delete;
use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::state::popup::ConfirmDelete;
use crate::app::state::popup::WipeConfirm;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    // Delete has [Delete] / [Cancel] buttons; Enter activates the focused one.
    if let Some(ConfirmDelete { cursor_idx, .. }) = state.dialogs.top_mut_as::<ConfirmDelete>() {
        match key.code {
            KeyCode::Left => *cursor_idx = 0,
            KeyCode::Right | KeyCode::Tab => *cursor_idx = 1 - (*cursor_idx).min(1),
            _ => {}
        }
    }
    let Some(confirmed) = confirm_answer(&key, false) else {
        return Err(());
    };
    match state.dialogs.pop() {
        // Enter on [Cancel] cancels like Esc (the selection stays).
        Some(PopupType::ConfirmDelete(ConfirmDelete {
            paths,
            cursor_idx: 0,
            to_trash,
            ..
        })) if confirmed => {
            delete::submit_delete(state, context, paths, to_trash);
        }
        Some(PopupType::WipeConfirm(WipeConfirm { paths })) if confirmed => {
            delete::submit_keeping_dialogs(
                state,
                TransferOperation::Wipe,
                paths,
                TransferOptions::default(),
                None,
            );
            delete::after_delete_submitted(state, context);
        }
        // Cancelled: the dialog underneath, if any (disk usage view), returns.
        _ => {}
    }
    Ok(None)
}

impl Dialog for ConfirmDelete {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
}

impl Dialog for WipeConfirm {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
}
