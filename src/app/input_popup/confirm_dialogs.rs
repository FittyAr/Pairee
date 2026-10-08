//! Yes/No confirmations (quit, interrupt transfers, reload / discard editor
//! changes, clear history, retry as administrator).

use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::state::history::HistoryKind;
use crate::app::state::{AdminOpKind, AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;
use std::path::PathBuf;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let letters = matches!(
        state.dialogs.top(),
        Some(PopupType::ConfirmDiscardEditorChanges)
    );
    let Some(confirmed) = confirm_answer(&key, letters) else {
        return Err(());
    };
    let Some(popup) = state.dialogs.pop() else {
        return Err(());
    };
    state.dialogs.clear();
    if confirmed {
        on_confirm(state, context, popup);
    } else if let PopupType::ConfirmClearHistory { history_type } = popup {
        reopen_history(state, &history_type);
    }
    Ok(None)
}

fn on_confirm(state: &mut AppState, context: &AppContext, popup: PopupType) {
    let show_hidden = context.config.settings.show_hidden;
    match popup {
        PopupType::ConfirmQuit => state.should_quit = true,
        PopupType::ConfirmInterrupt => {
            cancel_transfers(state);
            state.refresh_both_panels(show_hidden);
        }
        PopupType::ConfirmReload => crate::app::editor::open::reload_active_editor(state),
        PopupType::ConfirmDiscardEditorChanges => state.close_current_screen(),
        PopupType::ConfirmClearHistory { history_type } => clear_history(state, &history_type),
        PopupType::ConfirmRetryAsAdmin { paths, op_kind } => {
            if let Err(e) = retry_as_admin(state, paths, op_kind) {
                state.dialogs.replace(PopupType::Error(e));
            } else {
                state.refresh_both_panels(show_hidden);
            }
        }
        _ => {}
    }
}

/// Cancels the selected transfer job, or every active one.
fn cancel_transfers(state: &AppState) {
    let Some(ts) = &state.transfer else {
        return;
    };
    let jobs = ts.engine.queue.get_all();
    if let Some(job) = jobs.get(ts.queue_cursor) {
        job.cancel();
        ts.engine.queue.update_job(job.id, |j| {
            j.status = crate::fs::transfer::job::TransferJobStatus::Cancelled;
        });
    } else {
        jobs.iter()
            .filter(|j| j.is_active())
            .for_each(|j| j.cancel());
    }
}

fn clear_history(state: &mut AppState, history_type: &str) {
    if let Some(kind) = HistoryKind::from_key(history_type) {
        state.history.clear(kind);
    }
    state.history.save();
}

/// Esc on "clear history": back to the corresponding history list.
fn reopen_history(state: &mut AppState, history_type: &str) {
    if let Some(kind) = HistoryKind::from_key(history_type) {
        let list = kind.list_popup(&state.history, 0);
        state.dialogs.replace(list);
    }
}

/// Elevates and repeats the failed mkdir / rename. Returns the error message.
fn retry_as_admin(
    state: &mut AppState,
    paths: Vec<PathBuf>,
    op_kind: AdminOpKind,
) -> Result<(), String> {
    crate::fs::acquire_admin_privileges()
        .map_err(|e| format!("{} {}", t("error_acquire_admin_failed"), e))?;
    if cfg!(not(target_os = "windows")) {
        state.terminal_needs_clear = true;
    }
    match op_kind {
        AdminOpKind::MkDir => paths.iter().try_for_each(|path| {
            crate::fs::create_directory(path, true)
                .map_err(|e| format!("{} {}", t("error_mkdir_failed"), e))
        }),
        AdminOpKind::Rename { src, target } => {
            std::fs::rename(&src, &target).map_err(|e| format!("{} {}", t("error_rename_error"), e))
        }
    }
}
