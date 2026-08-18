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
    let popup = state.dialogs.top().cloned();
    if let Some(p) = popup {
        match p {
            PopupType::ConfirmQuit => {
                match key.code {
                    KeyCode::Enter => {
                        state.should_quit = true;
                        state.dialogs.clear();
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
            PopupType::ConfirmInterrupt => {
                match key.code {
                    KeyCode::Enter => {
                        // Cancel selected (or all active) transfer-engine jobs.
                        if let Some(ref ts) = state.transfer {
                            let jobs = ts.engine.queue.get_all();
                            if let Some(job) = jobs.get(ts.queue_cursor) {
                                job.is_cancelled
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                                ts.engine.queue.update_job(job.id, |j| {
                                    j.status =
                                        crate::fs::transfer::job::TransferJobStatus::Cancelled;
                                });
                            } else {
                                for job in jobs {
                                    if job.is_active() {
                                        job.is_cancelled
                                            .store(true, std::sync::atomic::Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                        state.dialogs.clear();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        // Keep transfer jobs running; just dismiss the confirm dialog.
                        state.dialogs.clear();
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
                                    state.dialogs.replace(PopupType::Error(format!(
                                        "{} {}",
                                        t("error_reload_failed"),
                                        e
                                    )));
                                    return Ok(None);
                                }
                            }
                        }
                        state.dialogs.clear();
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
            PopupType::ConfirmDiscardEditorChanges => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                        state.dialogs.clear();
                        state.close_current_screen();
                        return Ok(None);
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        state.dialogs.clear();
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
                            state.history.commands.clear();
                        } else if history_type == "view" {
                            state.history.viewed_files.clear();
                        } else if history_type == "folder" {
                            state.history.folders.clear();
                        }

                        // Save history store to disk
                        let history_store = crate::config::history::HistoryStore {
                            commands: state.history.commands.clone(),
                            viewed_files: state.history.viewed_files.clone(),
                            visited_folders: state.history.folders.clone(),
                        };
                        let _ = history_store.save();

                        state.dialogs.clear();
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        // Reopen the corresponding history list
                        if history_type == "command" {
                            state.dialogs.replace(PopupType::CommandHistoryList {
                                entries: state.history.commands.clone(),
                                cursor_idx: 0,
                            });
                        } else if history_type == "view" {
                            state.dialogs.replace(PopupType::FileViewHistoryList {
                                entries: state.history.viewed_files.clone(),
                                cursor_idx: 0,
                            });
                        } else if history_type == "folder" {
                            state.dialogs.replace(PopupType::FoldersHistoryList {
                                entries: state.history.folders.clone(),
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
                        state.dialogs.clear();

                        if let Err(e) = crate::fs::acquire_admin_privileges() {
                            state.dialogs.replace(PopupType::Error(format!(
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

                        match op_kind {
                            crate::app::state::AdminOpKind::MkDir => {
                                for path in &paths {
                                    if let Err(e) = crate::fs::create_directory(path, true) {
                                        state.dialogs.replace(PopupType::Error(format!(
                                            "{} {}",
                                            t("error_mkdir_failed"),
                                            e
                                        )));
                                        return Ok(None);
                                    }
                                }
                                state.refresh_both_panels(context.config.settings.show_hidden);
                            }
                            crate::app::state::AdminOpKind::Rename { src, target } => {
                                if let Err(e) = std::fs::rename(&src, &target) {
                                    state.dialogs.replace(PopupType::Error(format!(
                                        "{} {}",
                                        t("error_rename_error"),
                                        e
                                    )));
                                    return Ok(None);
                                }
                                state.refresh_both_panels(context.config.settings.show_hidden);
                            }
                        }
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
