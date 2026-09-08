use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::fs::transfer::events::TransferEvent;

pub fn handle_transfer_event(
    event: TransferEvent,
    state: &mut AppState,
    context: &AppContext,
    term_forwards: &mut Vec<(uuid::Uuid, Option<String>)>,
    refresh_needed: &mut bool,
) {
    let Some(ref mut transfer_state) = state.transfer else {
        return;
    };

    match event {
        TransferEvent::JobStarted { job_id } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.progress = Some(crate::fs::transfer::job::TransferProgress::default());
                job.results = crate::fs::transfer::job::TransferResults::default();
                job.log_lines.push(format!("[{}] Job started", job_id));
            });
        }
        TransferEvent::ScanStarted { job_id } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.status = crate::fs::transfer::job::TransferJobStatus::Scanning;
                job.progress = Some(crate::fs::transfer::job::TransferProgress::default());
                let msg = match job.operation {
                    crate::fs::transfer::job::TransferOperation::Delete => {
                        "Scanning source files for deletion...".to_string()
                    }
                    crate::fs::transfer::job::TransferOperation::Wipe => {
                        "Scanning source files for secure wipe...".to_string()
                    }
                    crate::fs::transfer::job::TransferOperation::Compress => {
                        "Preparing files for compression...".to_string()
                    }
                    crate::fs::transfer::job::TransferOperation::Extract => {
                        "Preparing archive for extraction...".to_string()
                    }
                    crate::fs::transfer::job::TransferOperation::ApplyCommand => {
                        "Preparing apply-command targets...".to_string()
                    }
                    _ => "Scanning source files...".to_string(),
                };
                job.log_lines.push(msg);
            });
        }
        TransferEvent::ScanProgress {
            job_id,
            files_found,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.files_scanned = files_found;
                }
            });
        }
        TransferEvent::ScanComplete {
            job_id,
            total_files,
            total_bytes,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.files_total = total_files;
                    prog.bytes_total = total_bytes;
                }
                job.log_lines.push(format!(
                    "Scan complete: {} files, {}",
                    total_files,
                    bytesize::ByteSize(total_bytes)
                ));
            });
        }
        TransferEvent::FileStarted {
            job_id,
            file,
            index,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.current_file = file.to_string_lossy().into_owned();
                }
                let msg = match job.operation {
                    crate::fs::transfer::job::TransferOperation::Delete => {
                        format!("[{}] Deleting: {}", index + 1, file.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Wipe => {
                        format!("[{}] Wiping: {}", index + 1, file.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Move => {
                        format!("[{}] Moving: {}", index + 1, file.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Compress => {
                        format!("[{}] Compressing: {}", index + 1, file.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Extract => {
                        format!("[{}] Extracting: {}", index + 1, file.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::ApplyCommand => {
                        format!("[{}] Applying: {}", index + 1, file.to_string_lossy())
                    }
                    _ => format!("[{}] Copying: {}", index + 1, file.to_string_lossy()),
                };
                job.log_lines.push(msg);
            });
        }
        TransferEvent::FileProgress {
            job_id,
            bytes_copied,
            bytes_total,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.bytes_transferred = bytes_copied;
                    prog.bytes_total = prog.bytes_total.max(bytes_total);
                }
            });
        }
        TransferEvent::CommandOutput { job_id, line } => {
            term_forwards.push((job_id, Some(line)));
        }
        TransferEvent::FileCompleted { job_id, result } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.files_completed += 1;
                }
                job.results.completed_files.push(result.clone());
                let msg = match job.operation {
                    crate::fs::transfer::job::TransferOperation::Delete => {
                        format!("✓ OK: Deleted {}", result.src.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Wipe => {
                        format!("✓ OK: Wiped {}", result.src.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Compress => {
                        format!("✓ OK: Packed {}", result.src.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::Extract => {
                        format!("✓ OK: Extracted {}", result.src.to_string_lossy())
                    }
                    crate::fs::transfer::job::TransferOperation::ApplyCommand => {
                        format!("✓ OK: Applied {}", result.src.to_string_lossy())
                    }
                    _ => {
                        let verified_marker = if result.verified { " ✓hash" } else { "" };
                        format!("✓ OK{}: {}", verified_marker, result.dst.to_string_lossy())
                    }
                };
                job.log_lines.push(msg);
            });
        }
        TransferEvent::FileFailed { job_id, error } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.files_failed += 1;
                }
                job.results.failed_files.push(error.clone());
                job.log_lines.push(format!(
                    "✗ FAIL: {} - {}",
                    error.src.to_string_lossy(),
                    error.error
                ));
            });
        }
        TransferEvent::FileSkipped {
            job_id,
            file,
            reason,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.files_skipped += 1;
                }
                job.results
                    .skipped_files
                    .push(crate::fs::transfer::job::SkippedFile {
                        src: file.clone(),
                        reason: reason.clone(),
                    });
                job.log_lines
                    .push(format!("⚠ SKIP: {} - {}", file.to_string_lossy(), reason));
            });
        }
        TransferEvent::SpeedUpdate {
            job_id,
            bytes_per_second,
            eta_seconds,
        } => {
            transfer_state.speed_info = (bytes_per_second, eta_seconds);
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.bytes_per_second = bytes_per_second;
                    prog.eta_seconds = eta_seconds;
                }
            });
        }
        TransferEvent::JobCompleted { results, job_id } => {
            term_forwards.push((job_id, None));
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.log_lines
                    .push(format!("[{}] Job completed successfully", job_id));
            });
            *refresh_needed = true;

            if context.config.settings.transfer_auto_report {
                let format = &context.config.settings.transfer_report_format;
                let content = if format == "csv" {
                    crate::fs::transfer::report::generate_csv_report(&results)
                } else {
                    crate::fs::transfer::report::generate_html_report(
                        &results,
                        &format!("Job {}", job_id),
                    )
                };
                let dest_dir = if let Some(first_file) = results.completed_files.first() {
                    first_file.dst.parent().unwrap_or(std::path::Path::new("."))
                } else {
                    std::path::Path::new(".")
                };
                if let Ok(report_path) =
                    crate::fs::transfer::report::save_report(&content, format, dest_dir)
                {
                    transfer_state.engine.queue.update_job(job_id, |job| {
                        job.log_lines.push(format!(
                            "Saved report to: {}",
                            report_path.to_string_lossy()
                        ));
                    });
                }
            }

            if transfer_state.engine.queue.pending_count() == 0
                && transfer_state.post_action != crate::fs::transfer::post_action::PostAction::None
            {
                let _ = crate::fs::transfer::post_action::execute_post_action(
                    transfer_state.post_action.clone(),
                );
            }
        }
        TransferEvent::JobFailed { error, job_id } => {
            term_forwards.push((job_id, None));
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.log_lines
                    .push(format!("[{}] Job failed: {}", job_id, error));
            });
            *refresh_needed = true;
        }
        TransferEvent::ConflictDetected {
            job_id,
            file,
            conflict,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.log_lines
                    .push(format!("Conflict detected: {}", file.to_string_lossy()));
            });
            transfer_state.active_conflict_info = Some((job_id, file, conflict));
            transfer_state.view_mode = crate::app::state::TransferViewMode::Expanded;
            state
                .dialogs
                .replace(crate::app::state::types::PopupType::TransferPanel);
            *refresh_needed = true;
        }
        TransferEvent::VerifyStarted {
            job_id,
            file,
            algorithm,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                job.log_lines.push(format!(
                    "🔍 Verifying [{}]: {}",
                    algorithm,
                    file.to_string_lossy()
                ));
            });
        }
        TransferEvent::VerifyProgress {
            job_id,
            bytes_verified,
            bytes_total,
        } => {
            transfer_state.engine.queue.update_job(job_id, |job| {
                if let Some(ref mut prog) = job.progress {
                    prog.bytes_transferred = bytes_verified;
                    prog.bytes_total = prog.bytes_total.max(bytes_total);
                }
            });
        }
    }
}
