//! Help browser: document list (mode 0) and document reader (mode 1).

use super::about::TEXT_SCROLL;
use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key, scroll_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::Help {
        mode,
        docs,
        plugin_docs,
        active_tab,
        cursor_idx,
        scroll_y,
        active_content,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match key.code {
        KeyCode::Esc => {
            state.dialogs.clear();
            return Ok(None);
        }
        KeyCode::Tab => {
            *mode = 1 - (*mode).min(1);
            return Ok(None);
        }
        _ => {}
    }
    if *mode != 0 {
        // Reader: scroll, Backspace returns to the list.
        if scroll_key(TEXT_SCROLL, key.code, scroll_y, usize::MAX) {
            return Ok(None);
        }
        return match key.code {
            KeyCode::Backspace => {
                *mode = 0;
                Ok(None)
            }
            _ => Err(()),
        };
    }
    let current: &[(String, PathBuf)] = if *active_tab == 0 { docs } else { plugin_docs };
    match list_key(ListKeys::ARROWS_VIM, key.code, cursor_idx, current.len()) {
        ListKey::Moved => {
            if let Some((_, path)) = current.get(*cursor_idx) {
                *active_content = std::fs::read_to_string(path).ok();
                *scroll_y = 0;
            }
        }
        ListKey::Activate(_) => *mode = 1,
        ListKey::Close => {}
        ListKey::Other => match key.code {
            KeyCode::Left | KeyCode::Right => {
                *active_tab = 1 - (*active_tab).min(1);
                *cursor_idx = 0;
                *scroll_y = 0;
                let next: &[(String, PathBuf)] = if *active_tab == 0 { docs } else { plugin_docs };
                *active_content = next
                    .first()
                    .and_then(|(_, path)| std::fs::read_to_string(path).ok());
            }
            _ => return Err(()),
        },
    }
    Ok(None)
}
