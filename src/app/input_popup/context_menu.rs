use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ContextMenu { items, cursor_idx }) = state.dialogs.top().cloned() {
        match key.code {
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            KeyCode::Up => {
                if !items.is_empty() {
                    let new_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        items.len() - 1
                    };
                    state.dialogs.replace(PopupType::ContextMenu {
                        items,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !items.is_empty() {
                    let new_idx = if cursor_idx < items.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.dialogs.replace(PopupType::ContextMenu {
                        items,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some(item) = items.get(cursor_idx) {
                    state.dialogs.clear();
                    if item.contains("View") {
                        return Ok(Some(Action::View));
                    } else if item.contains("Edit") {
                        return Ok(Some(Action::Edit));
                    } else if item.contains("Copy") {
                        return Ok(Some(Action::Copy));
                    } else if item.contains("Move") {
                        return Ok(Some(Action::Move));
                    } else if item.contains("Delete") {
                        return Ok(Some(Action::Delete));
                    } else if item.contains("Compress") {
                        return Ok(Some(Action::CompressFiles));
                    } else if item.contains("Extract") {
                        return Ok(Some(Action::ExtractArchive));
                    }
                }
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
