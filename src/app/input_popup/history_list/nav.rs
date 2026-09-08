//! Navigation helper for history lists (Up, Down, PgUp, PgDn, Home, End).

use crossterm::event::KeyCode;

pub fn handle_list_nav(code: KeyCode, cursor_idx: &mut usize, len: usize) -> bool {
    if len == 0 {
        return false;
    }
    match code {
        KeyCode::Up => {
            if *cursor_idx > 0 {
                *cursor_idx -= 1;
            } else {
                *cursor_idx = len - 1;
            }
            true
        }
        KeyCode::Down => {
            if *cursor_idx < len - 1 {
                *cursor_idx += 1;
            } else {
                *cursor_idx = 0;
            }
            true
        }
        KeyCode::PageUp => {
            *cursor_idx = cursor_idx.saturating_sub(10);
            true
        }
        KeyCode::PageDown => {
            *cursor_idx = (*cursor_idx + 10).min(len - 1);
            true
        }
        KeyCode::Home => {
            *cursor_idx = 0;
            true
        }
        KeyCode::End => {
            *cursor_idx = len - 1;
            true
        }
        _ => false,
    }
}
