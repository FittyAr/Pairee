//! Input handling for the command palette popup.

use crate::app::actions::command_palette::filter_items;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::CommandPalette {
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
            let new_idx = cursor_idx.saturating_sub(1);
            state.dialogs.replace(PopupType::CommandPalette {
                query,
                cursor_idx: new_idx,
                items,
            });
            Ok(None)
        }
        KeyCode::Down => {
            let max = items.len().saturating_sub(1);
            let new_idx = (cursor_idx + 1).min(max);
            state.dialogs.replace(PopupType::CommandPalette {
                query,
                cursor_idx: new_idx,
                items,
            });
            Ok(None)
        }
        KeyCode::Enter => {
            if let Some((_, action)) = items.get(cursor_idx) {
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
            let items = filter_items(&q);
            state.dialogs.replace(PopupType::CommandPalette {
                query: q,
                cursor_idx: 0,
                items,
            });
            Ok(None)
        }
        KeyCode::Char(c) => {
            let mut q = query;
            q.push(c);
            let items = filter_items(&q);
            state.dialogs.replace(PopupType::CommandPalette {
                query: q,
                cursor_idx: 0,
                items,
            });
            Ok(None)
        }
        _ => Ok(None),
    }
}
