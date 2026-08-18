use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.dialogs.top().cloned();
    if let Some(p) = popup {
        match p {
            PopupType::ConfirmDelete { paths, cursor_idx } => {
                match key.code {
                    KeyCode::Left => {
                        state.dialogs.replace(PopupType::ConfirmDelete {
                            paths,
                            cursor_idx: 0,
                        });
                        return Ok(None);
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        state.dialogs.replace(PopupType::ConfirmDelete {
                            paths,
                            cursor_idx: if cursor_idx == 0 { 1 } else { 0 },
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 0 {
                            let ssh_conn = state.get_active_panel().ssh_conn.clone();
                            let options = crate::fs::transfer::options::TransferOptions {
                                delete_to_recycle_bin: context
                                    .config
                                    .settings
                                    .delete_to_recycle_bin,
                                ..Default::default()
                            };
                            crate::fs::transfer::submit_simple(
                                state,
                                crate::fs::transfer::job::TransferOperation::Delete,
                                paths.clone(),
                                std::path::PathBuf::new(),
                                options,
                                ssh_conn,
                                None,
                            );
                        } else {
                            state.dialogs.clear();
                        }
                        state.get_active_panel_mut().clear_selection();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.dialogs.clear();
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            PopupType::WipeConfirm { paths } => {
                match key.code {
                    KeyCode::Enter => {
                        crate::fs::transfer::submit_simple(
                            state,
                            crate::fs::transfer::job::TransferOperation::Wipe,
                            paths,
                            std::path::PathBuf::new(),
                            crate::fs::transfer::options::TransferOptions::default(),
                            None,
                            None,
                        );
                        state.get_active_panel_mut().clear_selection();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.dialogs.clear();
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
