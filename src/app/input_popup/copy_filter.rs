use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::CopyMoveFilterPrompt {
        mut input,
        previous,
    }) = state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Esc => {
                state.dialogs.replace(*previous);
                return Ok(None);
            }
            KeyCode::Enter => {
                let mut prev = *previous;
                match prev {
                    PopupType::CopyPrompt {
                        ref mut filter_mask,
                        ..
                    } => {
                        *filter_mask = input;
                    }
                    PopupType::MovePrompt {
                        ref mut filter_mask,
                        ..
                    } => {
                        *filter_mask = input;
                    }
                    _ => {}
                }
                state.dialogs.replace(prev);
                return Ok(None);
            }
            KeyCode::Backspace => {
                input.pop();
                state
                    .dialogs
                    .replace(PopupType::CopyMoveFilterPrompt { input, previous });
                return Ok(None);
            }
            KeyCode::Char(c) => {
                input.push(c);
                state
                    .dialogs
                    .replace(PopupType::CopyMoveFilterPrompt { input, previous });
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
