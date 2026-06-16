use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const MAX_CURSOR_IDX: usize = 3; // 0=input, 1=process_multiple, 2=OK, 3=Cancel

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::MkDirPrompt {
        input,
        cursor_idx,
        process_multiple,
    }) = state.active_popup.clone()
    {
        let mut new_input = input.clone();
        let mut new_idx = cursor_idx;
        let mut new_multi = process_multiple;

        let update_popup = |s: &mut AppState, i: String, idx: usize, m: bool| {
            s.active_popup = Some(PopupType::MkDirPrompt {
                input: i,
                cursor_idx: idx,
                process_multiple: m,
            });
        };

        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                new_idx = if new_idx > 0 {
                    new_idx - 1
                } else {
                    MAX_CURSOR_IDX
                };
                update_popup(state, new_input, new_idx, new_multi);
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Tab => {
                new_idx = if new_idx < MAX_CURSOR_IDX {
                    new_idx + 1
                } else {
                    0
                };
                update_popup(state, new_input, new_idx, new_multi);
                return Ok(None);
            }
            KeyCode::Char(c) => {
                if new_idx == 0 {
                    new_input.push(c);
                    update_popup(state, new_input, new_idx, new_multi);
                } else if new_idx == 1 && c == ' ' {
                    new_multi = !new_multi;
                    update_popup(state, new_input, new_idx, new_multi);
                }
                return Ok(None);
            }
            KeyCode::Backspace => {
                if new_idx == 0 {
                    new_input.pop();
                    update_popup(state, new_input, new_idx, new_multi);
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if new_idx == 3 {
                    // Cancel
                    state.active_popup = None;
                    return Ok(None);
                }

                if !new_input.is_empty() {
                    let path = state.get_active_panel().current_path.join(new_input);
                    if let Err(e) = crate::fs::create_directory(
                        &path,
                        context.config.settings.req_admin_modification,
                    ) {
                        if !context.config.settings.req_admin_modification {
                            state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                paths: vec![path],
                                op_kind: crate::app::state::AdminOpKind::MkDir,
                            });
                        } else {
                            state.active_popup =
                                Some(PopupType::Error(format!("Directory error: {}", e)));
                        }
                    } else {
                        if context.config.settings.req_admin_modification {
                            state.terminal_needs_clear = true;
                        }
                        state.active_popup = None;
                        state.refresh_both_panels(context.config.settings.show_hidden);
                    }
                } else {
                    state.active_popup = None;
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::F(10) => {
                // Not implemented yet, just ignore so it doesn't quit
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
