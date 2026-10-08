//! Delete (F8) and wipe confirmations.

use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
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
    if let Some(PopupType::ConfirmDelete { cursor_idx, .. }) = state.dialogs.top_mut() {
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
        Some(PopupType::ConfirmDelete { paths, cursor_idx }) if confirmed => {
            if cursor_idx == 0 {
                let options = TransferOptions {
                    delete_to_recycle_bin: context.config.settings.delete_to_recycle_bin,
                    ..Default::default()
                };
                let ssh = state.get_active_panel().source.ssh().cloned();
                submit(
                    state,
                    context,
                    TransferOperation::Delete,
                    paths,
                    options,
                    ssh,
                );
            } else {
                after_submit(state, context);
            }
        }
        Some(PopupType::WipeConfirm { paths }) if confirmed => {
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
    ssh: Option<crate::fs::ssh::SharedSshClient>,
) {
    crate::app::actions::fs_ops::delete::submit_keeping_dialogs(
        state, operation, paths, options, ssh,
    );
    after_submit(state, context);
}

fn after_submit(state: &mut AppState, context: &AppContext) {
    state.get_active_panel_mut().clear_selection();
    state.refresh_both_panels(context.config.settings.show_hidden);
}
