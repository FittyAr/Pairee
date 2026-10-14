//! Yes/No confirmations (quit, interrupt transfers, reload / discard editor
//! changes, clear history, retry as administrator, undo/redo, save setup,
//! uninstall a plugin).
//! One generic key handler; each question says what its answers do.

use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::state::history::HistoryKind;
use crate::app::state::popup::{
    ConfirmClearHistory, ConfirmDiscardEditorChanges, ConfirmInterrupt, ConfirmQuit, ConfirmReload,
    ConfirmRetryAsAdmin, ConfirmUndo, ConfirmUninstallPlugin, Dialog, DialogKind, KeyHandler,
    KeyOutcome, SaveSetupConfirm,
};
use crate::app::state::{AdminOpKind, AppState, PopupType};
use crate::config::localization::t;
use crossterm::event::KeyEvent;
use std::path::PathBuf;

/// A Yes/No question: what Enter (or `y`) and Esc (or `n`) do. Every
/// dialog is closed before either runs.
pub trait Confirmation: DialogKind {
    /// `y` / `n` answer too (besides Enter / Esc).
    const LETTERS: bool = false;
    /// Asked over another dialog that stays open after the answer (only
    /// the question closes); otherwise every dialog closes.
    const NESTED: bool = false;

    fn confirm(self, state: &mut AppState, context: &mut AppContext);

    fn decline(self, _state: &mut AppState) {}
}

/// Keys of a confirmation of type `T`.
pub fn handle<T: Confirmation>(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> KeyOutcome {
    let Some(confirmed) = confirm_answer(&key, T::LETTERS) else {
        return Err(());
    };
    let Some(question) = state.dialogs.pop_as::<T>() else {
        return Err(());
    };
    if !T::NESTED {
        state.dialogs.clear();
    }
    if confirmed {
        question.confirm(state, context);
    } else {
        question.decline(state);
    }
    Ok(None)
}

macro_rules! confirmation_dialogs {
    ($($ty:ty),*) => {$(
        impl Dialog for $ty {
            fn key_handler(&self) -> KeyHandler {
                handle::<$ty>
            }
        }
    )*};
}
confirmation_dialogs!(
    ConfirmQuit,
    ConfirmUndo,
    ConfirmInterrupt,
    ConfirmReload,
    ConfirmDiscardEditorChanges,
    ConfirmClearHistory,
    ConfirmRetryAsAdmin,
    ConfirmUninstallPlugin,
    SaveSetupConfirm
);

impl Confirmation for ConfirmQuit {
    fn confirm(self, state: &mut AppState, _context: &mut AppContext) {
        state.should_quit = true;
    }
}

impl Confirmation for ConfirmUndo {
    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        crate::app::actions::fs_ops::undo::confirm(state, context, self.direction);
    }
}

/// Asked from the transfer panel, which is shown again after the answer.
impl Confirmation for ConfirmInterrupt {
    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        cancel_transfers(state);
        state.refresh_both_panels(context.config.settings.show_hidden);
        reopen_transfer_panel(state);
    }

    fn decline(self, state: &mut AppState) {
        reopen_transfer_panel(state);
    }
}

fn reopen_transfer_panel(state: &mut AppState) {
    if let Some(ts) = state.transfer.as_mut() {
        ts.view_mode = crate::app::state::TransferViewMode::Expanded;
        state
            .dialogs
            .replace(crate::app::state::popup::TransferPanel);
    }
}

impl Confirmation for ConfirmReload {
    fn confirm(self, state: &mut AppState, _context: &mut AppContext) {
        crate::app::editor::open::reload_active_editor(state);
    }
}

impl Confirmation for ConfirmDiscardEditorChanges {
    const LETTERS: bool = true;

    fn confirm(self, state: &mut AppState, _context: &mut AppContext) {
        state.close_current_screen();
    }
}

impl Confirmation for ConfirmClearHistory {
    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        if let Some(kind) = HistoryKind::from_key(&self.history_type) {
            state.history.clear(kind);
        }
        state.history.save(&context.config.settings);
    }

    /// Back to the corresponding history list.
    fn decline(self, state: &mut AppState) {
        if let Some(kind) = HistoryKind::from_key(&self.history_type) {
            let list = kind.list_popup(&state.history, 0);
            state.dialogs.replace(list);
        }
    }
}

impl Confirmation for ConfirmRetryAsAdmin {
    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        if let Err(e) = retry_as_admin(state, self.paths, self.op_kind) {
            state.dialogs.replace(PopupType::error(e));
        } else {
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
    }
}

/// Asked over the Plugin Manager, whose Installed rows are refreshed.
impl Confirmation for ConfirmUninstallPlugin {
    const LETTERS: bool = true;
    const NESTED: bool = true;

    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        super::plugin_menu::installed::uninstall(state, context, &self.name);
    }
}

impl Confirmation for SaveSetupConfirm {
    fn confirm(self, state: &mut AppState, context: &mut AppContext) {
        // An explicit "Save setup" is the user's confirmation that an
        // invalid config.toml (already backed up) may be replaced.
        context.config.confirm_settings_overwrite();
        crate::app::sys_helpers::capture_setup(state, &mut context.config.settings);
        let result = match context.config.save() {
            Ok(_) => PopupType::info(t("setup_saved_success")),
            Err(e) => PopupType::error(t("error_save_setup_failed").replace("{}", &e.to_string())),
        };
        state.dialogs.replace(result);
    }
}

/// Cancels the selected transfer job, or every active one.
fn cancel_transfers(state: &AppState) {
    let Some(ts) = &state.transfer else {
        return;
    };
    let jobs = ts.engine.queue.get_all();
    if let Some(job) = jobs.get(ts.queue_cursor) {
        super::transfer_panel::queue::cancel_job(ts, job);
    } else {
        jobs.iter()
            .filter(|j| j.is_active())
            .for_each(|j| j.cancel());
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
        state.redraw.needs_clear = true;
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
