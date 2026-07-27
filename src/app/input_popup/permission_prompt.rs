use crate::app::context::AppContext;
use crate::app::state::types::PermissionAnswer;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const BUTTONS: usize = 3;
const BUTTON_YES: usize = 0;
const BUTTON_NO: usize = 1;
const BUTTON_CANCEL: usize = 2;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::PermissionPrompt {
        paths,
        job_id,
        selected,
        sample_error,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                let next = if selected == 0 {
                    BUTTONS - 1
                } else {
                    selected - 1
                };
                state.active_popup = Some(PopupType::PermissionPrompt {
                    paths,
                    job_id,
                    selected: next,
                    sample_error,
                });
                Ok(None)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let next = if selected + 1 >= BUTTONS {
                    0
                } else {
                    selected + 1
                };
                state.active_popup = Some(PopupType::PermissionPrompt {
                    paths,
                    job_id,
                    selected: next,
                    sample_error,
                });
                Ok(None)
            }
            KeyCode::Tab => {
                let next = (selected + 1) % BUTTONS;
                state.active_popup = Some(PopupType::PermissionPrompt {
                    paths,
                    job_id,
                    selected: next,
                    sample_error,
                });
                Ok(None)
            }
            KeyCode::BackTab => {
                let next = if selected == 0 {
                    BUTTONS - 1
                } else {
                    selected - 1
                };
                state.active_popup = Some(PopupType::PermissionPrompt {
                    paths,
                    job_id,
                    selected: next,
                    sample_error,
                });
                Ok(None)
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                answer(state, paths, job_id, BUTTON_YES);
                Ok(None)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                answer(state, paths, job_id, BUTTON_NO);
                Ok(None)
            }
            KeyCode::Esc => {
                answer(state, paths, job_id, BUTTON_CANCEL);
                Ok(None)
            }
            KeyCode::Enter => {
                answer(state, paths, job_id, selected);
                Ok(None)
            }
            _ => Ok(None),
        }
    } else {
        Err(())
    }
}

/// Close the popup and store the user's choice on
/// `state.pending_permission_answer` so the background
/// loop can pick it up and drive the helper. We do not
/// run the helper inline because it spawns a
/// long-running process and blocks the UI thread.
fn answer(state: &mut AppState, paths: Vec<std::path::PathBuf>, job_id: uuid::Uuid, button: usize) {
    let answer = match button {
        BUTTON_YES => PermissionAnswer::Yes,
        BUTTON_NO => PermissionAnswer::No,
        _ => PermissionAnswer::Cancel,
    };
    state.pending_permission_answer = Some((job_id, paths, answer));
    state.active_popup = None;
}
