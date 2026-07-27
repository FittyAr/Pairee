use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ApplyCommandPrompt { input, targets }) = state.active_popup.clone() {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.active_popup = Some(PopupType::ApplyCommandPrompt {
                    input: new_input,
                    targets,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.active_popup = Some(PopupType::ApplyCommandPrompt {
                    input: new_input,
                    targets,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                state.active_popup = None;
                if !input.is_empty() {
                    // A10: apply_command is no longer
                    // available (it was a thin shim
                    // around the old ops_worker
                    // channel). The new engine does
                    // not yet have a generic
                    // "apply-command" operation; the
                    // command goes to the OS terminal
                    // instead. We log a warning and
                    // refresh the panels.
                    log::warn!(
                        "apply_command popup is no longer wired up (A10): '{}'",
                        input
                    );
                    let _ = targets;
                    state.refresh_both_panels(false);
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
