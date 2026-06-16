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
                    let rx = crate::fs::apply_command(input, targets);
                    state.progress_rx = Some(rx);
                    state.active_popup = Some(PopupType::CopyProgress {
                        is_move: false,
                        current_file: "Running command...".to_string(),
                        files_copied: 0,
                        total_files: 0,
                        bytes_copied: 0,
                        total_bytes: 0,
                    });
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
