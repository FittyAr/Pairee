//! Keys of the Synchronize folders dialog and of the folder-scan progress
//! popup.

mod options;
mod review;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() else {
        return Err(());
    };
    if dialog.review.is_some() {
        review::handle(state, key, context);
    } else {
        options::handle(state, key);
    }
    Ok(None)
}

/// Esc in the progress popup cancels the comparison.
pub fn handle_progress(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if key.code == KeyCode::Esc {
        crate::app::sync::cancel_scan(state);
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
