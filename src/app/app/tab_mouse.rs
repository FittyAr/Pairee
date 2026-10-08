//! Mouse on the tab bars: a left click shows the tab, a middle click
//! closes it.

use crate::app::state::{AppState, Screen};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Returns `true` when the event hit a tab.
pub fn handle_tab_bar_mouse(state: &mut AppState, mouse: MouseEvent, show_hidden: bool) -> bool {
    let on_panels = matches!(
        state.screens.get(state.active_screen_idx),
        Some(Screen::Panels)
    );
    if !on_panels || state.dialogs.is_some() {
        return false;
    }
    let MouseEventKind::Down(button) = mouse.kind else {
        return false;
    };
    let Some(id) = state.tab_bar.tab_at(mouse.column, mouse.row) else {
        return false;
    };
    match button {
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
