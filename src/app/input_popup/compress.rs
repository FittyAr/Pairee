use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::CompressPrompt {
        input,
        targets,
        dest_dir,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.active_popup = Some(PopupType::CompressPrompt {
                    input: new_input,
                    targets,
                    dest_dir,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.active_popup = Some(PopupType::CompressPrompt {
                    input: new_input,
                    targets,
                    dest_dir,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                if !input.is_empty() {
                    let mut out_name = input;
                    if !out_name.ends_with(".zip") {
                        out_name.push_str(".zip");
                    }
                    let final_dest = dest_dir.join(out_name);
                    let rx = crate::fs::spawn_compress_task(targets, final_dest);
                    state.progress_rx = Some(rx);
                    state.active_popup = Some(PopupType::CopyProgress {
                        is_move: false,
                        current_file: t("progress_compressing"),
                        files_copied: 0,
                        total_files: 0,
                        bytes_copied: 0,
                        total_bytes: 0,
                    });
                } else {
                    state.active_popup = None;
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
