//! Keys of the Compare files view: the diff viewer's keys; closing it stops
//! a comparison still running.

use super::diff_view::{DiffKey, handle_key};
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::{Dialog, FileCompareView, KeyHandler};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(view) = state.dialogs.top_mut_as::<FileCompareView>() else {
        return Err(());
    };
    if handle_key(&mut view.viewer, key.code) == DiffKey::Close {
        state.file_compare.cancel();
        state.dialogs.pop();
    }
    Ok(None)
}

impl Dialog for FileCompareView {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn resizable(&self) -> bool {
        true
    }
}
