use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::UserMenu) = state.active_popup {
        match key.code {
            KeyCode::Char('1') => {
                state.refresh_both_panels(context.config.settings.show_hidden);
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char('2') => {
                context.config.settings.show_hidden = !context.config.settings.show_hidden;
                let _ = context.config.save();
                state.refresh_both_panels(context.config.settings.show_hidden);
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char('3') => {
                state.swap_panels();
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char('4') => {
                return Ok(Some(Action::Help));
            }
            KeyCode::Char('5') | KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char('6') => {
                state.active_popup = None;
                let (tx, rx) = tokio::sync::mpsc::channel(100);
                tokio::spawn(async move {
                    let _ = tx
                        .send(crate::fs::ProgressUpdate {
                            current_file: "Downloading 7z...".to_string(),
                            files_copied: 0,
                            total_files: 1,
                            bytes_copied: 0,
                            total_bytes: 1,
                            error: None,
                        })
                        .await;

                    if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
                        let _ = tx
                            .send(crate::fs::ProgressUpdate {
                                current_file: "Completed".to_string(),
                                files_copied: 0,
                                total_files: 1,
                                bytes_copied: 0,
                                total_bytes: 1,
                                error: Some(format!("Failed to download: {}", e)),
                            })
                            .await;
                    } else {
                        let _ = tx
                            .send(crate::fs::ProgressUpdate {
                                current_file: "Completed".to_string(),
                                files_copied: 1,
                                total_files: 1,
                                bytes_copied: 1,
                                total_bytes: 1,
                                error: None,
                            })
                            .await;
                    }
                });

                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    is_move: false,
                    current_file: "Initializing Download...".to_string(),
                    files_copied: 0,
                    total_files: 1,
                    bytes_copied: 0,
                    total_bytes: 1,
                });
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
