//! Yes / No questions. Enter (or `y`) confirms, Esc (or `n`) declines;
//! what each answer does lives with the handler
//! (`app::input_popup::confirm_dialogs::Confirmation`).

use crate::app::state::types::AdminOpKind;
use std::path::PathBuf;

/// Quit Pairee?
#[derive(Debug, Clone)]
pub struct ConfirmQuit;

/// Interrupt the running transfers?
#[derive(Debug, Clone)]
pub struct ConfirmInterrupt;

/// Reload the edited file from disk, dropping the changes?
#[derive(Debug, Clone)]
pub struct ConfirmReload;

/// Close the editor without saving?
#[derive(Debug, Clone)]
pub struct ConfirmDiscardEditorChanges;

/// Save the current setup?
#[derive(Debug, Clone)]
pub struct SaveSetupConfirm;

/// Undo/redo of the newest journal entry: question and summary lines.
#[derive(Debug, Clone)]
pub struct ConfirmUndo {
    pub direction: crate::fs::journal::Direction,
    pub lines: Vec<String>,
}

/// Clear a history list (`HistoryKind` key)?
#[derive(Debug, Clone)]
pub struct ConfirmClearHistory {
    pub history_type: String,
}

/// Retry a failed operation with administrator rights?
#[derive(Debug, Clone)]
pub struct ConfirmRetryAsAdmin {
    pub paths: Vec<PathBuf>,
    pub op_kind: AdminOpKind,
}
