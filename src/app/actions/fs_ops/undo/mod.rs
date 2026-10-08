//! Undo / redo of file operations (**Files → Undo/Redo**, `Alt+Backspace` /
//! `Ctrl+Y`): asks for confirmation with a summary, then reverses the newest
//! journal entry through the regular operation paths ([`run`]).

mod record;
mod run;
mod summary;

#[cfg(test)]
mod tests;

pub use record::transfer_finished;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::journal::{Direction, check};

/// Opens the confirmation for the newest entry of `direction`, or explains
/// why there is nothing to do.
pub fn request(state: &mut AppState, direction: Direction) -> bool {
    let Some(entry) = state.journal.peek(direction) else {
        let key = match direction {
            Direction::Undo => "journal_nothing_to_undo",
            Direction::Redo => "journal_nothing_to_redo",
        };
        state.dialogs.replace(PopupType::Info(t(key)));
        return true;
    };
    let label = entry.label();
    let Some(inverse) = entry.inverse() else {
        state.journal.take(direction);
        let message = t("journal_not_undoable").replacen("{}", &label, 1);
        state.dialogs.replace(PopupType::Error(message));
        return true;
    };
    let checked = check(&inverse);
    let popup = match &checked.runnable {
        Some(runnable) => PopupType::ConfirmUndo {
            direction,
            lines: summary::confirm_lines(&label, direction, runnable, &checked.skipped),
        },
        None => {
            state.journal.take(direction);
            PopupType::Error(summary::nothing_left(&label, &checked.skipped))
        }
    };
    state.dialogs.replace(popup);
    true
}

/// Runs the confirmed undo/redo. The entry is checked again: the
/// filesystem may have changed while the dialog was open.
pub fn confirm(state: &mut AppState, context: &AppContext, direction: Direction) {
    let Some(entry) = state.journal.take(direction) else {
        return;
    };
    let Some(inverse) = entry.inverse() else {
        return;
    };
    let checked = check(&inverse);
    match checked.runnable {
        Some(runnable) => run::run(state, context, direction, runnable),
        None => {
            let message = summary::nothing_left(&entry.label(), &checked.skipped);
            state.dialogs.replace(PopupType::Error(message));
        }
    }
}
