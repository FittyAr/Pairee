//! Mouse selection in the editor: click places the cursor, drag selects,
//! `Alt`+drag selects a vertical block, the wheel scrolls.

use crate::app::state::AppState;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Lines scrolled per wheel notch.
const WHEEL_LINES: usize = 3;

/// Applies `mouse` to the active editor. Returns `true` when consumed.
pub fn handle_editor_mouse(state: &mut AppState, mouse: MouseEvent) -> bool {
    if state.dialogs.is_some() {
        return false;
    }
    let Some(ed) = state.active_editor_mut() else {
        return false;
    };
    let height = usize::from(ed.viewport.get().height);
    let (col, row) = (mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if ed.viewport.get().contains(col, row) => {
            let block = mouse.modifiers.contains(KeyModifiers::ALT);
            ed.mouse_press(col, row, block);
        }
        MouseEventKind::Drag(MouseButton::Left) if ed.selection.is_some() => {
            ed.mouse_drag(col, row);
            ed.ensure_cursor_visible(height);
        }
        MouseEventKind::ScrollUp => ed.scroll_y = ed.scroll_y.saturating_sub(WHEEL_LINES),
        MouseEventKind::ScrollDown => {
            let last = ed.lines.len().saturating_sub(height.max(1));
            ed.scroll_y = (ed.scroll_y + WHEEL_LINES).min(last);
        }
        _ => return false,
    }
    true
}
