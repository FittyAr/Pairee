//! Mouse on screens other than the panels: the close box of a screen and
//! the wheel over a terminal's output.

use crate::app::state::{AppState, Screen, ScreenId};
use crate::ui::hit::WHEEL_LINES;

/// A click on the close box of screen `id` (an editor with unsaved
/// changes asks first).
pub fn close(state: &mut AppState, id: ScreenId) -> bool {
    state.request_close_screen(id);
    true
}

/// A wheel notch over terminal screen `id` scrolls its output.
pub fn wheel(state: &mut AppState, id: ScreenId, down: bool) -> bool {
    match state.screens.by_id_mut(id) {
        Some(Screen::Terminal(ts)) => {
            ts.scroll_by(WHEEL_LINES, !down);
            true
        }
        _ => false,
    }
}
