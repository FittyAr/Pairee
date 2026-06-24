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
                        let is_move = matches!(
                            state.active_bg_op,
                            Some(crate::app::state::BackgroundOpContext::Move { .. })
                        );
                        state.active_popup = Some(PopupType::CopyProgress {
                            is_move,
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
                            let mut succeeded = true;
                            if src_paths.len() == 1 {
                                let dst = dest_dir.join(input.as_deref().unwrap_or_default());
                                if let Err(e) = crate::fs::rename_or_move_sync(
                                    &src_paths[0],
                                    &dst,
                                    context.config.settings.req_admin_modification,
                                ) {
                                    succeeded = false;
                                    if !context.config.settings.req_admin_modification {
                                        state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                            paths: src_paths.clone(),
                                            op_kind: crate::app::state::AdminOpKind::RenameMove {
                                                dst,
                                            },
                                        });
                                    } else {
                                        state.active_popup = Some(PopupType::Error(format!(
                                            "{} {}",
                                            t("error_move_failed"),
                                            e
                                        )));
                                    }
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
                                            succeeded = false;
                                            if !context.config.settings.req_admin_modification {
                                                state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                                    paths: src_paths.clone(),
                                                    op_kind: crate::app::state::AdminOpKind::RenameMove { dst: dest_dir.clone() },
                                                });
                                            } else {
                                                state.active_popup = Some(PopupType::Error(
                                                    format!("{} {}", t("error_move_failed"), e),
                                                ));
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            if succeeded && context.config.settings.req_admin_modification {
                                state.terminal_needs_clear = true;
                            }
                            state.get_active_panel_mut().clear_selection();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        } else {
                            let targets = src_paths;
                            let dest = if targets.len() == 1 {
                                dest_dir.join(input.as_deref().unwrap_or_default())
                            } else {
                                dest_dir
                            };

                            let rx = crate::fs::spawn_copy_task(
                                targets.clone(),
                                dest.clone(),
                                context.config.settings.clone(),
                            );
                            state.active_bg_op =
                                Some(crate::app::state::BackgroundOpContext::Copy {
                                    sources: targets,
                                    dest,
                                });
                            state.progress_rx = Some(rx);
                            state.active_popup = Some(PopupType::CopyProgress {
                                is_move: false,
                                current_file: t("progress_initializing"),
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

                        match op_kind {
                            crate::app::state::AdminOpKind::Delete => {
                                for path in &paths {
                                    if let Err(e) = crate::fs::delete_sync(
                                        path,
                                        context.config.settings.delete_to_recycle_bin,
                                        true,
                                    ) {
                                        state.active_popup = Some(PopupType::Error(format!(
                                            "{} {}",
                                            crate::config::localization::t("error_delete_failed"),
                                            e
                                        )));
                                        return Ok(None);
                                    }
                                }
                                state.get_active_panel_mut().clear_selection();
                                state.refresh_both_panels(context.config.settings.show_hidden);
                            }
                            crate::app::state::AdminOpKind::MkDir => {
                                for path in &paths {
                                    if let Err(e) = crate::fs::create_directory(path, true) {
                                        state.active_popup = Some(PopupType::Error(format!(
                                            "{} {}",
                                            t("error_mkdir_failed"),
                                            e
                                        )));
                                        return Ok(None);
                                    }
                                }
                                state.refresh_both_panels(context.config.settings.show_hidden);
                            }
                            crate::app::state::AdminOpKind::RenameMove { dst } => {
                                let mut settings = context.config.settings.clone();
                                settings.req_admin_modification = true;
                                let rx = crate::fs::spawn_move_task(
                                    paths.clone(),
                                    dst.clone(),
                                    settings,
                                );
                                state.active_bg_op =
                                    Some(crate::app::state::BackgroundOpContext::Move {
                                        sources: paths,
                                        dest: dst,
                                    });
                                state.progress_rx = Some(rx);
                                state.active_popup = Some(PopupType::CopyProgress {
                                    is_move: true,
                                    current_file: t("progress_initializing"),
                                    files_copied: 0,
                                    total_files: 0,
                                    bytes_copied: 0,
                                    total_bytes: 0,
                                });
                            }
                            crate::app::state::AdminOpKind::Copy { dst } => {
                                let mut settings = context.config.settings.clone();
                                settings.req_admin_modification = true;
                                let rx = crate::fs::spawn_copy_task(
                                    paths.clone(),
                                    dst.clone(),
                                    settings,
                                );
                                state.active_bg_op =
                                    Some(crate::app::state::BackgroundOpContext::Copy {
                                        sources: paths,
                                        dest: dst,
                                    });
                                state.progress_rx = Some(rx);
                                state.active_popup = Some(PopupType::CopyProgress {
                                    is_move: false,
                                    current_file: crate::config::localization::t(
                                        "progress_initializing",
                                    ),
                                    files_copied: 0,
                                    total_files: 0,
                                    bytes_copied: 0,
                                    total_bytes: 0,
                                });
                            }
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
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
