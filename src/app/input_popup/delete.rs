use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::ConfirmDelete { paths, cursor_idx } => {
                match key.code {
                    KeyCode::Left => {
                        state.active_popup = Some(PopupType::ConfirmDelete {
                            paths,
                            cursor_idx: 0,
                        });
                        return Ok(None);
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        state.active_popup = Some(PopupType::ConfirmDelete {
                            paths,
                            cursor_idx: if cursor_idx == 0 { 1 } else { 0 },
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 0 {
                            // Phase 6: unified engine handles Delete
                            // for both Local and SSH transparently.
                            use crate::fs::transfer::endpoint::TransferEndpoint;
                            use crate::fs::transfer::engine::TransferEngine;
                            use crate::fs::transfer::job::{TransferJob, TransferOperation};
                            use crate::fs::transfer::options::TransferOptions;

                            let mut options = TransferOptions::default();
                            options.delete_to_recycle_bin =
                                context.config.settings.delete_to_recycle_bin;

                            let endpoint = match &state.get_active_panel().ssh_conn {
                                Some(c) => TransferEndpoint::Ssh(c.clone()),
                                None => TransferEndpoint::Local,
                            };

                            let job = TransferJob::with_endpoints(
                                TransferOperation::Delete,
                                paths.clone(),
                                std::path::PathBuf::new(),
                                options,
                                endpoint.clone(),
                                endpoint,
                            );

                            if state.transfer.is_none() {
                                let (engine, rx) = TransferEngine::new();
                                state.transfer =
                                    Some(crate::app::state::transfer_state::TransferUIState::new(
                                        engine, rx,
                                    ));
                            }

                            if let Some(ref mut ts) = state.transfer {
                                ts.engine.submit_job(job);
                                ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                            }
                            state.active_popup = None;
                            state.get_active_panel_mut().clear_selection();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                            return Ok(None);
                        } else {
                            state.active_popup = None;
                        }
                        state.get_active_panel_mut().clear_selection();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            PopupType::WipeConfirm { paths } => {
                match key.code {
                    KeyCode::Enter => {
                        // Phase 9: secure wipe goes through the
                        // unified engine with wipe_passes = 3.
                        crate::app::actions::fs_ops::wipe::commit(state, paths);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
