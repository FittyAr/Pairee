//! The progress popup of a Git network operation.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::{Dialog, GitProgress, KeyHandler, KeyOutcome};
use crossterm::event::{KeyCode, KeyEvent};

/// Esc cancels the running operation.
fn handle(state: &mut AppState, key: KeyEvent, _context: &mut AppContext) -> KeyOutcome {
    if key.code == KeyCode::Esc {
        state.cancel_git_op();
    }
    Ok(None)
}

impl Dialog for GitProgress {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
}
