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
                        state.active_popup = Some(PopupType::CopyProgress {
                            current_file: "Resuming...".to_string(),
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
            PopupType::ConfirmOverwrite {
                src_paths,
                dest_dir,
                is_move,
                input,
            } => {
                match key.code {
                    KeyCode::Enter => {
                        state.active_popup = None;

                        if is_move {
                            if src_paths.len() == 1 {
                                let dst = dest_dir.join(input.as_deref().unwrap_or_default());
                                if let Err(e) = crate::fs::rename_or_move_sync(
                                    &src_paths[0],
                                    &dst,
                                    context.config.settings.req_admin_modification,
                                ) {
                                    state.active_popup =
                                        Some(PopupType::Error(format!("Move failed: {}", e)));
                                }
                            } else {
                                for src in &src_paths {
                                    if let Some(fname) = src.file_name() {
                                        let dst = dest_dir.join(fname);
                                        if let Err(e) = crate::fs::rename_or_move_sync(
                                            src,
                                            &dst,
                                            context.config.settings.req_admin_modification,
                                        ) {
                                            state.active_popup = Some(PopupType::Error(format!(
                                                "Move failed: {}",
                                                e
                                            )));
                                            break;
                                        }
                                    }
                                }
                            }
                            state.get_active_panel_mut().selected_paths.clear();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        } else {
                            let targets = src_paths;
                            let dest = if targets.len() == 1 {
                                dest_dir.join(input.as_deref().unwrap_or_default())
                            } else {
                                dest_dir
                            };

                            let rx = crate::fs::spawn_copy_task(
                                targets,
                                dest,
                                context.config.settings.clone(),
                            );
                            state.progress_rx = Some(rx);
                            state.active_popup = Some(PopupType::CopyProgress {
                                current_file: "Initializing...".to_string(),
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
            }
            PopupType::ConfirmReload {
                path,
                lines,
                cursor_x,
                cursor_y,
                scroll_y,
                is_dirty,
                last_search,
            } => {
                match key.code {
                    KeyCode::Enter => {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let reloaded_lines: Vec<String> =
                                    content.lines().map(|s| s.to_string()).collect();
                                state.active_popup = Some(PopupType::InternalEditor {
                                    path,
                                    lines: if reloaded_lines.is_empty() {
                                        vec![String::new()]
                                    } else {
                                        reloaded_lines
                                    },
                                    cursor_x: cursor_x.min(
                                        content.lines().nth(cursor_y).map(|l| l.len()).unwrap_or(0),
                                    ),
                                    cursor_y,
                                    scroll_y,
                                    is_dirty: false,
                                    last_search,
                                });
                            }
                            Err(e) => {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Failed to reload: {}", e)));
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        // Return to editor with unchanged state
                        state.active_popup = Some(PopupType::InternalEditor {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                        });
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
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
