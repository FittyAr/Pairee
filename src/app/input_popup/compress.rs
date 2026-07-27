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

                    // A5: enqueue a Compress job on the
                    // unified transfer engine. The
                    // legacy `fs::spawn_compress_task`
                    // and the `state.progress_rx`
                    // channel are no longer used.
                    use crate::app::state::transfer_state::TransferUIState;
                    use crate::fs::transfer::engine::TransferEngine;
                    use crate::fs::transfer::job::{ArchiveFormat, TransferJob, TransferOperation};
                    use crate::fs::transfer::options::TransferOptions;

                    if state.transfer.is_none() {
                        let (engine, rx) = TransferEngine::new();
                        state.transfer = Some(TransferUIState::new(engine, rx));
                    }
                    if let Some(ref mut ts) = state.transfer {
                        let options = TransferOptions::default();
                        let job = TransferJob::with_endpoints(
                            TransferOperation::Compress {
                                format: ArchiveFormat::Zip,
                                level: 6,
                            },
                            targets,
                            final_dest,
                            options,
                            crate::fs::transfer::endpoint::TransferEndpoint::Local,
                            crate::fs::transfer::endpoint::TransferEndpoint::Local,
                        );
                        ts.engine.submit_job(job);
                        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                    }
                    let _ = t("progress_compressing");
                    state.active_popup = None;
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
