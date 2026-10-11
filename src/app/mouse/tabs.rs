//! Mouse on the tab bars: a left click shows the tab, a middle click
//! closes it.

use super::Click;
use crate::app::state::AppState;
use crate::app::state::tabs::TabId;
use crossterm::event::MouseButton;

/// Returns `true` when the click did something to tab `id`.
pub fn click(state: &mut AppState, id: TabId, click: Click, show_hidden: bool) -> bool {
    match click.button {
        MouseButton::Left => {
            state.activate_tab(id, show_hidden);
        }
        MouseButton::Middle => {
            state.close_tab(id, show_hidden);
        }
        MouseButton::Right => return false,
    }
    true
}
