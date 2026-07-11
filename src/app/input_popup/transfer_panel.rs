use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferViewMode};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let transfer = match &mut state.transfer {
        Some(t) => t,
        None => return Err(()),
    };

    if let Some((job_id, _, _)) = transfer.active_conflict_info {
        let resolution = match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') => Some(crate::fs::transfer::conflict::ConflictResolution::Overwrite),
            KeyCode::Char('a') | KeyCode::Char('A') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(crate::fs::transfer::conflict::ConflictResolution::Skip),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(crate::fs::transfer::conflict::ConflictResolution::Rename),
            _ => None,
        };

        if let Some(res) = resolution {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
                let mut guard = job.active_conflict.lock().unwrap();
                *guard = Some(res);
            }
            transfer.active_conflict_info = None;
            return Ok(None);
        }
        if key.code != KeyCode::Esc {
            return Ok(None);
        }
    }

    match key.code {
        KeyCode::Esc => {
            // Minimizar a barra compacta
            transfer.view_mode = TransferViewMode::Minimized;
            state.active_popup = None;
            Ok(None)
        }
        KeyCode::Tab => {
            // Siguiente pestaña
            let next_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::Queue,
                TransferTab::Queue => TransferTab::FileList,
            };
            transfer.active_tab = next_tab;
            Ok(None)
        }
        KeyCode::BackTab => {
            // Pestaña anterior
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Queue,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
                TransferTab::Queue => TransferTab::Log,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Left => {
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Queue,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
                TransferTab::Queue => TransferTab::Log,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Right => {
            let next_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::Queue,
                TransferTab::Queue => TransferTab::FileList,
            };
            transfer.active_tab = next_tab;
            Ok(None)
        }
        KeyCode::Char('1') => {
            transfer.active_tab = TransferTab::FileList;
            Ok(None)
        }
        KeyCode::Char('2') => {
            transfer.active_tab = TransferTab::Options;
            Ok(None)
        }
        KeyCode::Char('3') => {
            transfer.active_tab = TransferTab::Status;
            Ok(None)
        }
        KeyCode::Char('4') => {
            transfer.active_tab = TransferTab::Log;
            Ok(None)
        }
        KeyCode::Char('5') => {
            transfer.active_tab = TransferTab::Queue;
            Ok(None)
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            // Alternar Pausa / Reanudación
            let is_paused = transfer.engine.queue.get_active()
                .map(|j| j.status == crate::fs::transfer::job::TransferJobStatus::Paused)
                .unwrap_or(false);

            if is_paused {
                transfer.engine.resume();
            } else {
                transfer.engine.pause();
            }
            Ok(None)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Saltar archivo
            transfer.engine.skip_file();
            Ok(None)
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Cancelar
            transfer.engine.cancel();
            Ok(None)
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(ref res) = transfer.current_results {
                if !res.failed_files.is_empty() {
                    let failed_sources: Vec<std::path::PathBuf> = res.failed_files.iter().map(|f| f.src.clone()).collect();
                    if let Some(active_job) = transfer.engine.queue.get_active() {
                        let new_job = crate::fs::transfer::job::TransferJob::new(
                            active_job.operation,
                            failed_sources,
                            active_job.destination.clone(),
                            active_job.options.clone(),
                        );
                        transfer.log_lines.push(format!("Re-enqueueing {} failed files...", res.failed_files.len()));
                        transfer.engine.submit_job(new_job);
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(ref res) = transfer.current_results {
                let content = if _context.config.settings.transfer_report_format == "csv" {
                    crate::fs::transfer::report::generate_csv_report(res)
                } else {
                    crate::fs::transfer::report::generate_html_report(res, "Manual Export")
                };
                let format = _context.config.settings.transfer_report_format.clone();
                let dest_dir = if let Some(first_file) = res.completed_files.first() {
                    first_file.dst.parent().unwrap_or(std::path::Path::new("."))
                } else {
                    std::path::Path::new(".")
                };
                if let Ok(report_path) = crate::fs::transfer::report::save_report(&content, &format, dest_dir) {
                    transfer.log_lines.push(format!("Manually saved report to: {}", report_path.to_string_lossy()));
                }
            }
            Ok(None)
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if transfer.active_tab == TransferTab::Queue {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    if transfer.engine.queue.reorder(job.id, -1) {
                        transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
                        transfer.log_lines.push(format!("[{}] Moved up in queue", job.id));
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Char('-') => {
            if transfer.active_tab == TransferTab::Queue {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    if transfer.engine.queue.reorder(job.id, 1) {
                        if transfer.queue_cursor < jobs.len().saturating_sub(1) {
                            transfer.queue_cursor += 1;
                        }
                        transfer.log_lines.push(format!("[{}] Moved down in queue", job.id));
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if transfer.active_tab == TransferTab::Queue {
                transfer.engine.queue.clear_completed();
                transfer.queue_cursor = 0;
                transfer.log_lines.push("Cleared completed and terminal jobs from queue".to_string());
            }
            Ok(None)
        }
        KeyCode::Delete => {
            if transfer.active_tab == TransferTab::Queue {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    if transfer.engine.queue.remove(job.id) {
                        transfer.log_lines.push(format!("[{}] Job removed from queue", job.id));
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Up => {
            if transfer.active_tab == TransferTab::FileList {
                transfer.file_list_cursor = transfer.file_list_cursor.saturating_sub(1);
            } else if transfer.active_tab == TransferTab::Queue {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            } else if transfer.active_tab == TransferTab::Options {
                transfer.options_cursor = transfer.options_cursor.saturating_sub(1);
            }
            Ok(None)
        }
        KeyCode::Down => {
            if transfer.active_tab == TransferTab::FileList {
                let total_files = transfer.current_results.as_ref()
                    .map(|res| res.failed_files.len() + res.skipped_files.len() + res.completed_files.len())
                    .unwrap_or(0);
                let max_idx = total_files.saturating_sub(1);
                if transfer.file_list_cursor < max_idx {
                    transfer.file_list_cursor += 1;
                }
            } else if transfer.active_tab == TransferTab::Queue {
                let max_idx = transfer.engine.queue.get_all().len().saturating_sub(1);
                if transfer.queue_cursor < max_idx {
                    transfer.queue_cursor += 1;
                }
            } else if transfer.active_tab == TransferTab::Options {
                if transfer.options_cursor < 6 {
                    transfer.options_cursor += 1;
                }
            }
            Ok(None)
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if transfer.active_tab == TransferTab::Options {
                match transfer.options_cursor {
                    0 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.direct_io = !opts.direct_io;
                        });
                    }
                    1 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.verify_after_copy = !opts.verify_after_copy;
                        });
                    }
                    2 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.preserve_timestamps = !opts.preserve_timestamps;
                        });
                    }
                    3 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.preserve_attributes = !opts.preserve_attributes;
                        });
                    }
                    4 => {
                        // Alternar post action en la UI
                        transfer.post_action = match transfer.post_action {
                            crate::fs::transfer::post_action::PostAction::None => crate::fs::transfer::post_action::PostAction::Shutdown,
                            crate::fs::transfer::post_action::PostAction::Shutdown => crate::fs::transfer::post_action::PostAction::Sleep,
                            crate::fs::transfer::post_action::PostAction::Sleep => crate::fs::transfer::post_action::PostAction::Hibernate,
                            crate::fs::transfer::post_action::PostAction::Hibernate => crate::fs::transfer::post_action::PostAction::CloseApp,
                            crate::fs::transfer::post_action::PostAction::CloseApp => crate::fs::transfer::post_action::PostAction::None,
                        };
                    }
                    5 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.buffer_size = match opts.buffer_size {
                                crate::fs::transfer::options::BufferSize::_64KB => crate::fs::transfer::options::BufferSize::_256KB,
                                crate::fs::transfer::options::BufferSize::_256KB => crate::fs::transfer::options::BufferSize::_1MB,
                                crate::fs::transfer::options::BufferSize::_1MB => crate::fs::transfer::options::BufferSize::_4MB,
                                crate::fs::transfer::options::BufferSize::_4MB => crate::fs::transfer::options::BufferSize::_64KB,
                            };
                        });
                    }
                    6 => {
                        transfer.engine.queue.update_active_options(|opts| {
                            opts.hash_algorithm = match opts.hash_algorithm {
                                crate::fs::transfer::options::HashAlgorithm::Blake3 => crate::fs::transfer::options::HashAlgorithm::Crc32,
                                crate::fs::transfer::options::HashAlgorithm::Crc32 => crate::fs::transfer::options::HashAlgorithm::Md5,
                                crate::fs::transfer::options::HashAlgorithm::Md5 => crate::fs::transfer::options::HashAlgorithm::Sha1,
                                crate::fs::transfer::options::HashAlgorithm::Sha1 => crate::fs::transfer::options::HashAlgorithm::Sha256,
                                crate::fs::transfer::options::HashAlgorithm::Sha256 => crate::fs::transfer::options::HashAlgorithm::Blake3,
                            };
                        });
                    }
                    _ => {}
                }
            }
            Ok(None)
        }
        _ => Err(()),
    }
}
