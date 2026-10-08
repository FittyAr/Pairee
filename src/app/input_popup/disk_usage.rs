//! Keys of the disk usage view.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::AppState;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let du = &mut state.disk_usage;
    let len = du.current().map_or(0, |dir| dir.children.len());
    match list_key(ListKeys::FULL, key.code, &mut du.cursor, len) {
        ListKey::Moved => {}
        ListKey::Activate(_) => {
            du.enter();
        }
        ListKey::Close => close(state),
        ListKey::Other => match key.code {
            KeyCode::Right => {
                du.enter();
            }
            KeyCode::Left | KeyCode::Backspace => {
                du.leave();
            }
            KeyCode::Char('r' | 'R') | KeyCode::F(5) => du.rescan(),
            KeyCode::Delete | KeyCode::F(8) => delete_selected(state, context),
            KeyCode::F(10) => close(state),
            _ => {}
        },
    }
    Ok(None)
}

/// Stops a running scan and closes the view (a finished tree stays cached).
fn close(state: &mut AppState) {
    state.disk_usage.cancel();
    state.dialogs.pop();
}

/// Sends the highlighted item to the regular delete flow (confirmation,
/// recycle bin setting, transfer queue); the view drops it once it is gone.
fn delete_selected(state: &mut AppState, context: &AppContext) {
    if state.disk_usage.is_scanning() {
        return;
    }
    let Some(path) = state.disk_usage.selected_path() else {
        return;
    };
    state.disk_usage.watch_delete(path.clone());
    crate::app::actions::fs_ops::delete::request(state, context, vec![path], true);
}
