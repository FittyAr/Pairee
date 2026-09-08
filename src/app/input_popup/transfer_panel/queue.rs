//! Queue management actions for transfer panel (pause, cancel, retry, export, reorder).

use crate::app::context::AppContext;
use crate::app::state::{DialogStack, TransferUIState};
use crossterm::event::KeyCode;

pub fn handle_queue_action(
    transfer: &mut TransferUIState,
    dialogs: &mut DialogStack,
    context: &AppContext,
    code: KeyCode,
) -> bool {
    match code {
        KeyCode::Char('p') | KeyCode::Char('P') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let is_paused = job.status == crate::fs::transfer::job::TransferJobStatus::Paused;
                if is_paused {
                    for other_job in &jobs {
                        if other_job.id != job.id {
                            other_job
                                .is_paused
                                .store(true, std::sync::atomic::Ordering::SeqCst);
                            transfer.engine.queue.update_job(other_job.id, |j| {
                                if j.status
                                    == crate::fs::transfer::job::TransferJobStatus::Transferring
                                    || j.status
                                        == crate::fs::transfer::job::TransferJobStatus::Scanning
                                    || j.status
                                        == crate::fs::transfer::job::TransferJobStatus::Verifying
                                {
                                    j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                                }
                            });
                        }
                    }
                    job.is_paused
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.status = crate::fs::transfer::job::TransferJobStatus::Transferring;
                    });
                } else if job.status == crate::fs::transfer::job::TransferJobStatus::Transferring
                    || job.status == crate::fs::transfer::job::TransferJobStatus::Scanning
                    || job.status == crate::fs::transfer::job::TransferJobStatus::Verifying
                {
                    job.is_paused
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                    });
                } else if job.status == crate::fs::transfer::job::TransferJobStatus::Queued {
                    for other_job in &jobs {
                        if other_job.id != job.id {
                            other_job
                                .is_paused
                                .store(true, std::sync::atomic::Ordering::SeqCst);
                            transfer.engine.queue.update_job(other_job.id, |j| {
                                if j.status
                                    == crate::fs::transfer::job::TransferJobStatus::Transferring
                                    || j.status
                                        == crate::fs::transfer::job::TransferJobStatus::Scanning
                                    || j.status
                                        == crate::fs::transfer::job::TransferJobStatus::Verifying
                                {
                                    j.status = crate::fs::transfer::job::TransferJobStatus::Paused;
                                }
                            });
                        }
                    }
                    job.is_paused
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.reorder(job.id, -(jobs.len() as i32));
                }
            }
            true
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                job.skip_file_flag
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            true
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if context
                .config
                .settings
                .confirmations
                .confirm_interrupt_operation
            {
                dialogs.replace(crate::app::state::PopupType::ConfirmInterrupt);
            } else {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    job.is_cancelled
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.status = crate::fs::transfer::job::TransferJobStatus::Cancelled;
                    });
                }
            }
            true
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let res = &job.results;
                if !res.failed_files.is_empty() {
                    let failed_sources: Vec<std::path::PathBuf> =
                        res.failed_files.iter().map(|f| f.src.clone()).collect();
                    let new_job = crate::fs::transfer::job::TransferJob::new(
                        job.operation,
                        failed_sources,
                        job.destination.clone(),
                        job.options.clone(),
                    );
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.log_lines.push(format!(
                            "Re-enqueueing {} failed files...",
                            res.failed_files.len()
                        ));
                    });
                    transfer.engine.submit_job(new_job);
                }
            }
            true
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor) {
                let res = &job.results;
                let content = if context.config.settings.transfer_report_format == "csv" {
                    crate::fs::transfer::report::generate_csv_report(res)
                } else {
                    crate::fs::transfer::report::generate_html_report(res, "Manual Export")
                };
                let format = context.config.settings.transfer_report_format.clone();
                let dest_dir = if let Some(first_file) = res.completed_files.first() {
                    first_file.dst.parent().unwrap_or(std::path::Path::new("."))
                } else {
                    std::path::Path::new(".")
                };
                if let Ok(report_path) =
                    crate::fs::transfer::report::save_report(&content, &format, dest_dir)
                {
                    transfer.engine.queue.update_job(job.id, |j| {
                        j.log_lines.push(format!(
                            "Manually saved report to: {}",
                            report_path.to_string_lossy()
                        ));
                    });
                }
            }
            true
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor)
                && transfer.engine.queue.reorder(job.id, -1)
            {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            }
            true
        }
        KeyCode::Char('-') => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor)
                && transfer.engine.queue.reorder(job.id, 1)
                && transfer.queue_cursor < jobs.len().saturating_sub(1)
            {
                transfer.queue_cursor += 1;
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            transfer.engine.queue.clear_completed();
            transfer.queue_cursor = 0;
            true
        }
        KeyCode::Delete => {
            let jobs = transfer.engine.queue.get_all();
            if let Some(job) = jobs.get(transfer.queue_cursor)
                && transfer.engine.queue.remove(job.id)
            {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            }
            true
        }
        _ => false,
    }
}
