use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ApplyCommandPrompt { input, targets }) = state.dialogs.top().cloned() {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.dialogs.replace(PopupType::ApplyCommandPrompt {
                    input: new_input,
                    targets,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.dialogs.replace(PopupType::ApplyCommandPrompt {
                    input: new_input,
                    targets,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                state.dialogs.clear();
                if !input.is_empty() {
                    crate::fs::transfer::submit_apply_command(state, input, targets);
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
