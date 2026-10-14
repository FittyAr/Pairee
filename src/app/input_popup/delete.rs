//! Delete (F8) and wipe confirmations.

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
        Some(PopupType::ConfirmDelete(ConfirmDelete {
            paths,
            cursor_idx,
            to_trash,
            ..
        })) if confirmed => {
            if cursor_idx == 0 {
                let options = TransferOptions {
                    delete_to_recycle_bin: to_trash,
                    ..Default::default()
                };
                let remote = state.get_active_panel().source.remote().cloned();
                submit(
                    state,
                    context,
                    TransferOperation::Delete,
                    paths,
                    options,
                    remote,
                );
            } else {
                after_submit(state, context);
            }
        }
        Some(PopupType::WipeConfirm(WipeConfirm { paths })) if confirmed => {
            let options = TransferOptions::default();
            submit(
                state,
                context,
                TransferOperation::Wipe,
                paths,
                options,
                None,
            );
        }
        // Cancelled: the dialog underneath, if any (disk usage view), returns.
        _ => {}
    }
    Ok(None)
}

fn submit(
    state: &mut AppState,
    context: &AppContext,
    operation: TransferOperation,
    paths: Vec<std::path::PathBuf>,
    options: TransferOptions,
    remote: Option<crate::fs::remote::RemoteConn>,
) {
    crate::app::actions::fs_ops::delete::submit_keeping_dialogs(
        state, operation, paths, options, remote,
    );
    after_submit(state, context);
}

fn after_submit(state: &mut AppState, context: &AppContext) {
    state.get_active_panel_mut().clear_selection();
    state.refresh_both_panels(context.config.settings.show_hidden);
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
