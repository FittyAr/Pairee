use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::About { mut scroll_y }) = state.dialogs.top().cloned() {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                state.dialogs.clear();
                return Ok(None);
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                scroll_y = scroll_y.saturating_sub(1);
                state.dialogs.replace(PopupType::About { scroll_y });
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                scroll_y = scroll_y.saturating_add(1);
                state.dialogs.replace(PopupType::About { scroll_y });
                return Ok(None);
            }
            KeyCode::PageUp => {
                scroll_y = scroll_y.saturating_sub(15);
                state.dialogs.replace(PopupType::About { scroll_y });
                return Ok(None);
            }
            KeyCode::PageDown => {
                scroll_y = scroll_y.saturating_add(15);
                state.dialogs.replace(PopupType::About { scroll_y });
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
