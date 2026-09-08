//! Input handling for the which-key overlay.

use crate::app::actions::which_key::filter_items;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::WhichKey {
        query,
        cursor_idx,
        items,
    }) = state.dialogs.top().cloned()
    else {
        return Err(());
    };

    match key.code {
        KeyCode::Esc => {
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Up => {
            let visible = filter_items(&query, &items);
            let new_idx = cursor_idx.saturating_sub(1);
            let new_idx = if visible.is_empty() {
                0
            } else {
                new_idx.min(visible.len() - 1)
            };
            state.dialogs.replace(PopupType::WhichKey {
                query,
                cursor_idx: new_idx,
                items,
            });
            Ok(None)
        }
        KeyCode::Down => {
            let visible = filter_items(&query, &items);
            let max = visible.len().saturating_sub(1);
            let new_idx = (cursor_idx + 1).min(max);
            state.dialogs.replace(PopupType::WhichKey {
                query,
                cursor_idx: new_idx,
                items,
            });
            Ok(None)
        }
        KeyCode::Enter => {
            let visible = filter_items(&query, &items);
            let idx = cursor_idx.min(visible.len().saturating_sub(1));
            if let Some((_, _, action)) = visible.get(idx) {
                let action = *action;
                state.dialogs.clear();
                return Ok(Some(action));
            }
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Backspace => {
            let mut q = query;
            q.pop();
            state.dialogs.replace(PopupType::WhichKey {
                query: q,
                cursor_idx: 0,
                items,
            });
            Ok(None)
        }
        KeyCode::Char(c) => {
            let mut q = query;
            q.push(c);
            state.dialogs.replace(PopupType::WhichKey {
                query: q,
                cursor_idx: 0,
                items,
            });
            Ok(None)
        }
        _ => Ok(None),
    }
}
