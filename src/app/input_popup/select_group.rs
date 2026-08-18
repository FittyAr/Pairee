use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, SelectMode};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SelectGroupPrompt { mode, query }) = state.dialogs.top().cloned() {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_q = query;
                new_q.push(c);
                state
                    .dialogs
                    .replace(PopupType::SelectGroupPrompt { mode, query: new_q });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_q = query;
                new_q.pop();
                state
                    .dialogs
                    .replace(PopupType::SelectGroupPrompt { mode, query: new_q });
                return Ok(None);
            }
            KeyCode::Enter => {
                state.dialogs.clear();
                match mode {
                    SelectMode::Add => state.get_active_panel_mut().select_group(&query),
                    SelectMode::Remove => state.get_active_panel_mut().unselect_group(&query),
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
