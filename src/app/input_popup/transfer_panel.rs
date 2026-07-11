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
            KeyCode::Char('o') => Some(crate::fs::transfer::conflict::ConflictResolution::Overwrite),
            KeyCode::Char('O') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteAll),
            KeyCode::Char('a') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder),
            KeyCode::Char('A') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll),
            KeyCode::Char('s') => Some(crate::fs::transfer::conflict::ConflictResolution::Skip),
            KeyCode::Char('S') => Some(crate::fs::transfer::conflict::ConflictResolution::SkipAll),
            KeyCode::Char('r') => Some(crate::fs::transfer::conflict::ConflictResolution::Rename),
            KeyCode::Char('R') => Some(crate::fs::transfer::conflict::ConflictResolution::RenameAll),
            KeyCode::Char('x') | KeyCode::Char('X') => Some(crate::fs::transfer::conflict::ConflictResolution::Cancel),
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
        KeyCode::Char('t') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            Ok(Some(Action::ToggleTransferPanel))
        }
        KeyCode::Char('T') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            Ok(Some(Action::ToggleTransferPanel))
        }
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
                TransferTab::Log => TransferTab::FileList,
            };
            transfer.active_tab = next_tab;
            Ok(None)
        }
        KeyCode::BackTab => {
            // Pestaña anterior
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Log,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Left => {
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Log,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Right => {
            let next_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::FileList,
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
        KeyCode::Char('p') | KeyCode::Char('P') => {
            // Alternar Pausa / Reanudación sobre el trabajo seleccionado
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let is_paused = job.status == crate::fs::transfer::job::TransferJobStatus::Paused;
                if is_paused {
                    // Reanudar el seleccionado
                    // 1. Pausar todos los demás trabajos activos
                    for other_job in &jobs {
                        if other_job.id != job.id {
                            other_job.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
                            transfer.engine.queue.update_job(other_job.id, |j| {
                                if j.status == crate::fs::transfer::job::TransferJobStatus::Transferring 
                                    || j.status == crate::fs::transfer::job::TransferJobStatus::Scanning 
                                    || j.status == crate::fs::transfer::job::TransferJobStatus::Verifying 
                                {
                                    j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                                }
                            });
                        }
                    }
                    // 2. Activar el seleccionado
                    job.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.status = crate::fs::transfer::job::TransferJobStatus::Transferring;
                    });
                } else if job.status == crate::fs::transfer::job::TransferJobStatus::Transferring 
                    || job.status == crate::fs::transfer::job::TransferJobStatus::Scanning 
                    || job.status == crate::fs::transfer::job::TransferJobStatus::Verifying 
                {
                    // Pausar
                    job.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                    });
                } else if job.status == crate::fs::transfer::job::TransferJobStatus::Queued {
                    // Pausar los demás
                    for other_job in &jobs {
                        if other_job.id != job.id {
                            other_job.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
                            transfer.engine.queue.update_job(other_job.id, |j| {
                                if j.status == crate::fs::transfer::job::TransferJobStatus::Transferring 
                                    || j.status == crate::fs::transfer::job::TransferJobStatus::Scanning 
                                    || j.status == crate::fs::transfer::job::TransferJobStatus::Verifying 
                                {
                                    j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                                }
                            });
                        }
                    }
                    // Promover/ejecutar el seleccionado
                    job.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
                    // Movemos al principio de la cola para que el coordinador lo tome de inmediato
                    transfer.engine.queue.reorder(job.id, -(jobs.len() as i32));
                }
            }
            Ok(None)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Saltar archivo de la tarea seleccionada
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                job.skip_file_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Ok(None)
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Cancelar la tarea seleccionada
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                job.is_cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
                transfer.engine.queue.update_job(job.id, |j| {
                    j.status = crate::fs::transfer::job::TransferJobStatus::Cancelled;
                });
            }
            Ok(None)
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let res = &job.results;
                if !res.failed_files.is_empty() {
                    let failed_sources: Vec<std::path::PathBuf> = res.failed_files.iter().map(|f| f.src.clone()).collect();
                    let new_job = crate::fs::transfer::job::TransferJob::new(
                        job.operation,
                        failed_sources,
                        job.destination.clone(),
                        job.options.clone(),
                    );
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.log_lines.push(format!("Re-enqueueing {} failed files...", res.failed_files.len()));
                    });
                    transfer.engine.submit_job(new_job);
                }
            }
            Ok(None)
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let res = &job.results;
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
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.log_lines.push(format!("Manually saved report to: {}", report_path.to_string_lossy()));
                    });
                }
            }
            Ok(None)
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                if transfer.engine.queue.reorder(job.id, -1) {
                    transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
                }
            }
            Ok(None)
        }
        KeyCode::Char('-') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                if transfer.engine.queue.reorder(job.id, 1) {
                    if transfer.queue_cursor < jobs.len().saturating_sub(1) {
                        transfer.queue_cursor += 1;
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            transfer.engine.queue.clear_completed();
            transfer.queue_cursor = 0;
            Ok(None)
        }
        KeyCode::Delete => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                if transfer.engine.queue.remove(job.id) {
                    transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
                }
            }
            Ok(None)
        }
        KeyCode::Up => {
            transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            Ok(None)
        }
        KeyCode::Down => {
            let max_idx = transfer.engine.queue.get_all().len().saturating_sub(1);
            if transfer.queue_cursor < max_idx {
                transfer.queue_cursor += 1;
            }
            Ok(None)
        }
        KeyCode::PageUp => {
            if transfer.active_tab == TransferTab::FileList {
                transfer.file_list_cursor = transfer.file_list_cursor.saturating_sub(1);
            } else if transfer.active_tab == TransferTab::Options {
                transfer.options_cursor = transfer.options_cursor.saturating_sub(1);
            }
            Ok(None)
        }
        KeyCode::PageDown => {
            if transfer.active_tab == TransferTab::FileList {
                let jobs = transfer.engine.queue.get_all();
                let total_files = if let Some(job) = jobs.get(transfer.queue_cursor) {
                    job.results.failed_files.len() + job.results.skipped_files.len() + job.results.completed_files.len()
                } else {
                    0
                };
                let max_idx = total_files.saturating_sub(1);
                if transfer.file_list_cursor < max_idx {
                    transfer.file_list_cursor += 1;
                }
            } else if transfer.active_tab == TransferTab::Options {
                if transfer.options_cursor < 11 {
                    transfer.options_cursor += 1;
                }
            }
            Ok(None)
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if transfer.active_tab == TransferTab::Options {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    let job_id = job.id;
                    transfer.engine.queue.update_job(job_id, |j| {
                        match transfer.options_cursor {
                            0 => j.options.direct_io = !j.options.direct_io,
                            1 => j.options.verify_after_copy = !j.options.verify_after_copy,
                            2 => j.options.preserve_timestamps = !j.options.preserve_timestamps,
                            3 => j.options.preserve_attributes = !j.options.preserve_attributes,
                            4 => {}
                            5 => {
                                j.options.buffer_size = match j.options.buffer_size {
                                    crate::fs::transfer::options::BufferSize::_64KB => crate::fs::transfer::options::BufferSize::_256KB,
                                    crate::fs::transfer::options::BufferSize::_256KB => crate::fs::transfer::options::BufferSize::_1MB,
                                    crate::fs::transfer::options::BufferSize::_1MB => crate::fs::transfer::options::BufferSize::_4MB,
                                    crate::fs::transfer::options::BufferSize::_4MB => crate::fs::transfer::options::BufferSize::_64KB,
                                };
                            }
                            6 => {
                                j.options.hash_algorithm = match j.options.hash_algorithm {
                                    crate::fs::transfer::options::HashAlgorithm::Blake3 => crate::fs::transfer::options::HashAlgorithm::Crc32,
                                    crate::fs::transfer::options::HashAlgorithm::Crc32 => crate::fs::transfer::options::HashAlgorithm::Md5,
                                    crate::fs::transfer::options::HashAlgorithm::Md5 => crate::fs::transfer::options::HashAlgorithm::Sha1,
                                    crate::fs::transfer::options::HashAlgorithm::Sha1 => crate::fs::transfer::options::HashAlgorithm::Sha256,
                                    crate::fs::transfer::options::HashAlgorithm::Sha256 => crate::fs::transfer::options::HashAlgorithm::Blake3,
                                };
                            }
                            7 => j.options.preserve_acl = !j.options.preserve_acl,
                            8 => j.options.preserve_streams = !j.options.preserve_streams,
                            9 => j.options.skip_symlinks = !j.options.skip_symlinks,
                            10 => j.options.follow_symlinks = !j.options.follow_symlinks,
                            11 => {
                                j.options.limit_bandwidth_rate = match j.options.limit_bandwidth_rate {
                                    None => Some(1_048_576),
                                    Some(1_048_576) => Some(10_485_760),
                                    Some(10_485_760) => Some(52_428_800),
                                    Some(_) => None,
                                };
                            }
                            _ => {}
                        }
                    });
                    if transfer.options_cursor == 4 {
                        transfer.post_action = match &transfer.post_action {
                            crate::fs::transfer::post_action::PostAction::None => crate::fs::transfer::post_action::PostAction::Shutdown,
                            crate::fs::transfer::post_action::PostAction::Shutdown => crate::fs::transfer::post_action::PostAction::Sleep,
                            crate::fs::transfer::post_action::PostAction::Sleep => crate::fs::transfer::post_action::PostAction::Hibernate,
                            crate::fs::transfer::post_action::PostAction::Hibernate => crate::fs::transfer::post_action::PostAction::EjectDrive(String::new()),
                            crate::fs::transfer::post_action::PostAction::EjectDrive(_) => crate::fs::transfer::post_action::PostAction::RunScript(std::path::PathBuf::new()),
                            crate::fs::transfer::post_action::PostAction::RunScript(_) => crate::fs::transfer::post_action::PostAction::CloseApp,
                            crate::fs::transfer::post_action::PostAction::CloseApp => crate::fs::transfer::post_action::PostAction::None,
                        };
                    }
                }
            }
            Ok(None)
        }
        _ => Err(()),
    }
}
