use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
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
            PopupType::ConfirmQuit => {
                match key.code {
                    KeyCode::Enter => {
                        state.should_quit = true;
                        state.active_popup = None;
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
            PopupType::ConfirmInterrupt => {
                match key.code {
                    KeyCode::Enter => {
                        state.progress_rx = None;
                        state.active_popup = None;
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        // Resume the progress popup; it will automatically receive progress updates on the next tick
                        // Phase 5: Copy and Move now flow through the
                        // engine's own TransferPanel popup, not the
                        // legacy CopyProgress. Only the Delete path
                        // still uses CopyProgress, and Delete is
                        // never an is_move operation.
                        state.active_popup = Some(PopupType::CopyProgress {
                            is_move: false,
                            current_file: t("progress_resuming"),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                        });
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }

            PopupType::ConfirmReload => {
                match key.code {
                    KeyCode::Enter => {
                        if let Some(crate::app::state::Screen::Editor(ed)) =
                            state.screens.get_mut(state.active_screen_idx)
                        {
                            match std::fs::read_to_string(&ed.path) {
                                Ok(content) => {
                                    let reloaded_lines: Vec<String> =
                                        content.lines().map(|s| s.to_string()).collect();
                                    ed.lines = if reloaded_lines.is_empty() {
                                        vec![String::new()]
                                    } else {
                                        reloaded_lines
                                    };
                                    ed.cursor_x = ed.cursor_x.min(
                                        ed.lines.get(ed.cursor_y).map(|l| l.len()).unwrap_or(0),
                                    );
                                    ed.is_dirty = false;
                                }
                                Err(e) => {
                                    state.active_popup = Some(PopupType::Error(format!(
                                        "{} {}",
                                        t("error_reload_failed"),
                                        e
                                    )));
                                    return Ok(None);
                                }
                            }
                        }
                        state.active_popup = None;
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
            PopupType::ConfirmDiscardEditorChanges => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                        state.active_popup = None;
                        state.close_current_screen();
                        return Ok(None);
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            PopupType::ConfirmClearHistory { history_type } => {
                match key.code {
                    KeyCode::Enter => {
                        if history_type == "command" {
                            state.command_history.clear();
                        } else if history_type == "view" {
                            state.file_view_history.clear();
                        } else if history_type == "folder" {
                            state.folders_history.clear();
                        }

                        // Save history store to disk
                        let mut history_store = crate::config::history::HistoryStore::default();
                        history_store.commands = state.command_history.clone();
                        history_store.viewed_files = state.file_view_history.clone();
                        history_store.visited_folders = state.folders_history.clone();
                        let _ = history_store.save();

                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        // Reopen the corresponding history list
                        if history_type == "command" {
                            state.active_popup = Some(PopupType::CommandHistoryList {
                                entries: state.command_history.clone(),
                                cursor_idx: 0,
                            });
                        } else if history_type == "view" {
                            state.active_popup = Some(PopupType::FileViewHistoryList {
                                entries: state.file_view_history.clone(),
                                cursor_idx: 0,
                            });
                        } else if history_type == "folder" {
                            state.active_popup = Some(PopupType::FoldersHistoryList {
                                entries: state.folders_history.clone(),
                                cursor_idx: 0,
                            });
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            PopupType::ConfirmRetryAsAdmin { paths, op_kind } => {
                match key.code {
                    KeyCode::Enter => {
                        state.active_popup = None;

                        // Cache sudo credentials so the upcoming `sudo`
                        // re-exec inside `run_in_elevated_helper` does not
                        // need to re-prompt. This used to be the *whole*
                        // elevation logic, which silently failed because
                        // the calling process kept running as the user;
                        // now we also actually re-exec the operation
                        // below.
                        if let Err(e) = crate::fs::acquire_admin_privileges() {
                            state.active_popup = Some(PopupType::Error(format!(
                                "{} {}",
                                t("error_acquire_admin_failed"),
                                e
                            )));
                            return Ok(None);
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            state.terminal_needs_clear = true;
                        }

                        // Build the elevated operation list and run it via
                        // `run_in_elevated_helper`, which writes the ops
                        // to a temp JSON file and re-launches the current
                        // binary under `sudo` (or with `RunAs` on
                        // Windows). The helper performs the operations
                        // with the elevated process and writes a result
                        // file we read here. This is the only correct way
                        // to do admin work in this code base.
                        let ops: Vec<crate::fs::FsOperation> = match op_kind {
                            crate::app::state::AdminOpKind::MkDir => paths
                                .iter()
                                .map(|p| crate::fs::FsOperation::MkDir { path: p.clone() })
                                .collect(),
                            crate::app::state::AdminOpKind::Rename { src, target } => {
                                vec![crate::fs::FsOperation::Move {
                                    src: src.clone(),
                                    dst: target.clone(),
                                }]
                            }
                        };

                        if let Err(e) = crate::fs::run_in_elevated_helper(ops) {
                            state.active_popup = Some(PopupType::Error(format!(
                                "{} {}",
                                t("error_elevated_helper_failed"),
                                e
                            )));
                            return Ok(None);
                        }

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
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
