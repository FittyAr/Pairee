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
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = Some(*previous);
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
                state.active_popup = Some(prev);
                return Ok(None);
            }
            KeyCode::Backspace => {
                input.pop();
                state.active_popup = Some(PopupType::CopyMoveFilterPrompt { input, previous });
                return Ok(None);
            }
            KeyCode::Char(c) => {
                input.push(c);
                state.active_popup = Some(PopupType::CopyMoveFilterPrompt { input, previous });
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
