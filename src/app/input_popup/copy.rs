use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::CopyPrompt {
        input,
        src_paths,
        dest_dir,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.active_popup = Some(PopupType::CopyPrompt {
                    input: new_input,
                    src_paths,
                    dest_dir,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.active_popup = Some(PopupType::CopyPrompt {
                    input: new_input,
                    src_paths,
                    dest_dir,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                // Check for overwrite confirmation if enabled
                if context.config.settings.confirmations.confirm_overwrite {
                    let mut any_exists = false;
                    if src_paths.len() == 1 {
                        let dst = dest_dir.join(&input);
                        if dst.exists() {
                            any_exists = true;
                        }
                    } else {
                        for src in &src_paths {
                            if let Some(fname) = src.file_name() {
                                let dst = dest_dir.join(fname);
                                if dst.exists() {
                                    any_exists = true;
                                    break;
                                }
                            }
                        }
                    }

                    if any_exists {
                        state.active_popup = Some(PopupType::ConfirmOverwrite {
                            src_paths,
                            dest_dir,
                            is_move: false,
                            input: Some(input),
                        });
                        return Ok(None);
                    }
                }

                // If no overwrite check needed or no files exist, copy directly
                state.active_popup = None;

                let targets = src_paths;
                let dest = if targets.len() == 1 {
                    dest_dir.join(&input)
                } else {
                    dest_dir
                };

                let rx = crate::fs::spawn_copy_task(targets, dest, context.config.settings.clone());
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    current_file: "Initializing...".to_string(),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                });

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
