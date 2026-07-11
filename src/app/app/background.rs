use crate::app::context::AppContext;
use crate::app::state::{AppState, DevProgress, PopupType, Screen};
use crate::terminal::TerminalBackend;

pub fn process_background_updates(
    state: &mut AppState,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) {
    // 1. Process background operation updates (e.g. copy progress)
    if state.progress_rx.is_some() {
        let mut rx = state.progress_rx.take().unwrap();
        let mut is_completed = false;
        let mut has_error = None;
        let mut latest_update = None;

        while let Ok(update) = rx.try_recv() {
            if let Some(err) = update.error.clone() {
                has_error = Some(err);
            } else if update.current_file == "Completed" {
                is_completed = true;
            } else {
                latest_update = Some(update);
            }
        }

        if let Some(err) = has_error {
            if !context.config.settings.req_admin_modification {
                match state.active_bg_op.take() {
                    Some(crate::app::state::BackgroundOpContext::Copy { sources, dest }) => {
                        state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                            paths: sources,
                            op_kind: crate::app::state::AdminOpKind::Copy { dst: dest },
                        });
                    }
                    Some(crate::app::state::BackgroundOpContext::Move { sources, dest }) => {
                        state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                            paths: sources,
                            op_kind: crate::app::state::AdminOpKind::RenameMove { dst: dest },
                        });
                    }
                    None => {
                        state.active_popup = Some(PopupType::Error(err));
                    }
                }
            } else {
                state.active_popup = Some(PopupType::Error(err));
                state.active_bg_op = None;
            }
        } else if is_completed {
            state.active_popup = None;
            state.active_bg_op = None;
            state.refresh_both_panels(context.config.settings.show_hidden);
        } else {
            if let Some(update) = latest_update {
                let should_update = match &state.active_popup {
                    None | Some(PopupType::CopyProgress { .. }) => true,
                    _ => false,
                };
                if should_update {
                    // Preserve the is_move flag from the current popup if present
                    let is_move = match &state.active_popup {
                        Some(PopupType::CopyProgress { is_move, .. }) => *is_move,
                        _ => matches!(
                            state.active_bg_op,
                            Some(crate::app::state::BackgroundOpContext::Move { .. })
                        ),
                    };
                    state.active_popup = Some(PopupType::CopyProgress {
                        is_move,
                        current_file: update.current_file,
                        files_copied: update.files_copied,
                        total_files: update.total_files,
                        bytes_copied: update.bytes_copied,
                        total_bytes: update.total_bytes,
                    });
                }
            }
            state.progress_rx = Some(rx);
        }
    }

    // 1.5 Process Terminal background updates
    if state.term_rx.is_some() {
        let mut rx = state.term_rx.take().unwrap();
        while let Ok(update) = rx.try_recv() {
            if let Some(Screen::Terminal(ts)) = state.screens.get_mut(update.screen_idx) {
                match update.line {
                    Some(line) => ts.output_lines.push(line),
                    None => ts.is_running = false,
                }
            }
        }
        state.term_rx = Some(rx);
    }

    // 1.6 Process background SSH connection attempts
    if state.ssh_connect_rx.is_some() {
        let mut rx = state.ssh_connect_rx.take().unwrap();
        match rx.try_recv() {
            Ok((panel, res)) => match res {
                Ok(client) => {
                    let p = match panel {
                        crate::app::state::ActivePanel::Left => &mut state.left_panel,
                        crate::app::state::ActivePanel::Right => &mut state.right_panel,
                    };
                    p.ssh_conn = Some(client);
                    p.current_path = std::path::PathBuf::from("/");
                    p.cursor_index = 0;
                    p.clear_selection();
                    state.active_popup = None;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                }
                Err(e) => {
                    state.active_popup = Some(PopupType::Error(format!(
                        "{} {}",
                        crate::config::localization::t("error_ssh_failed"),
                        e
                    )));
                }
            },
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                state.ssh_connect_rx = Some(rx);
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {}
        }
    }

    // 1.7 Process background search updates
    if state.search_rx.is_some() {
        let mut rx = state.search_rx.take().unwrap();
        let mut new_results = Vec::new();
        let mut closed = false;
        loop {
            match rx.try_recv() {
                Ok((path, is_dir)) => {
                    new_results.push((path, is_dir));
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    break;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }
        if !new_results.is_empty() {
            if let Some(PopupType::SearchResults { results, .. }) = &mut state.active_popup {
                for (path, is_dir) in new_results {
                    if results.len() < 500 {
                        results.push((path, is_dir));
                    } else {
                        closed = true;
                        break;
                    }
                }
            }
        }
        if closed {
            if let Some(PopupType::SearchResults { searching, .. }) = &mut state.active_popup {
                *searching = false;
            }
        } else {
            state.search_rx = Some(rx);
        }
    }

    // 1.8 Process Developer Tools progress updates (async init/lint/package/install/submit)
    if state.dev_progress_rx.is_some() {
        let mut rx = state.dev_progress_rx.take().unwrap();
        let mut latest: Option<DevProgress> = None;
        let mut finished: Option<DevProgress> = None;
        let mut disconnected = false;
        loop {
            match rx.try_recv() {
                Ok(update) => {
                    if update.done {
                        // The terminal message supersedes any in-flight ones.
                        finished = Some(update);
                        latest = None;
                    } else {
                        latest = Some(update);
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if let Some(update) = finished {
            if let Some(PopupType::PluginMenu {
                dev_results,
                dev_loading,
                dev_loading_status,
                dev_loading_progress,
                ..
            }) = &mut state.active_popup
            {
                if let Some(err) = update.error {
                    *dev_results = err;
                } else if let Some(res) = update.result {
                    *dev_results = res;
                }
                *dev_loading = false;
                *dev_loading_status = String::new();
                *dev_loading_progress = None;
            }
        } else if let Some(update) = latest {
            if let Some(PopupType::PluginMenu {
                dev_loading,
                dev_loading_status,
                dev_loading_progress,
                ..
            }) = &mut state.active_popup
            {
                *dev_loading = true;
                *dev_loading_status = update.status;
                *dev_loading_progress = if let (Some(c), Some(t)) = (update.current, update.total) {
                    if t > 0 { Some((c, t)) } else { None }
                } else {
                    None
                };
            }
        }
        if !disconnected {
            state.dev_progress_rx = Some(rx);
        }
    }

    // 1.9 Process Transfer Engine events
    let mut refresh_needed = false;
    if let Some(ref mut transfer_state) = state.transfer {
        while let Ok(event) = transfer_state.event_rx.try_recv() {
            use crate::fs::transfer::events::TransferEvent;
            
            match event {
                TransferEvent::JobStarted { job_id } => {
                    transfer_state.log_lines.push(format!("[{}] Job started", job_id));
                    transfer_state.current_progress = Some(crate::fs::transfer::job::TransferProgress::default());
                    transfer_state.current_results = Some(crate::fs::transfer::job::TransferResults::default());
                }
                TransferEvent::ScanProgress { job_id, files_found } => {
                    let _ = job_id; // reserved for multi-job routing
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.files_scanned = files_found;
                    }
                }
                TransferEvent::ScanComplete { job_id, total_files, total_bytes } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.files_total = total_files;
                        prog.bytes_total = total_bytes;
                    }
                    transfer_state.log_lines.push(format!("Scan complete: {} files, {}", total_files, bytesize::ByteSize(total_bytes)));
                }
                TransferEvent::FileStarted { job_id, file, index } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.current_file = file.to_string_lossy().into_owned();
                    }
                    transfer_state.log_lines.push(format!("[{}] Copying: {}", index + 1, file.to_string_lossy()));
                }
                TransferEvent::FileProgress { job_id, bytes_copied, bytes_total } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.bytes_transferred = bytes_copied;
                        prog.bytes_total = prog.bytes_total.max(bytes_total);
                    }
                }
                TransferEvent::FileCompleted { job_id, result } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.files_completed += 1;
                    }
                    if let Some(ref mut res) = transfer_state.current_results {
                        res.completed_files.push(result.clone());
                    }
                    let verified_marker = if result.verified { " ✓hash" } else { "" };
                    transfer_state.log_lines.push(format!("✓ OK{}: {}", verified_marker, result.dst.to_string_lossy()));
                }
                TransferEvent::FileFailed { job_id, error } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.files_failed += 1;
                    }
                    if let Some(ref mut res) = transfer_state.current_results {
                        res.failed_files.push(error.clone());
                    }
                    transfer_state.log_lines.push(format!("✗ FAIL: {} - {}", error.src.to_string_lossy(), error.error));
                }
                TransferEvent::FileSkipped { job_id, file, reason } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.files_skipped += 1;
                    }
                    if let Some(ref mut res) = transfer_state.current_results {
                        res.skipped_files.push(crate::fs::transfer::job::SkippedFile {
                            src: file.clone(),
                            reason: reason.clone(),
                        });
                    }
                    transfer_state.log_lines.push(format!("⚠ SKIP: {} - {}", file.to_string_lossy(), reason));
                }
                TransferEvent::SpeedUpdate { job_id, bytes_per_second, eta_seconds } => {
                    let _ = job_id;
                    transfer_state.speed_info = (bytes_per_second, eta_seconds);
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.bytes_per_second = bytes_per_second;
                        prog.eta_seconds = eta_seconds;
                    }
                }
                TransferEvent::JobCompleted { results, job_id } => {
                    transfer_state.log_lines.push(format!("[{}] Job completed successfully", job_id));
                    transfer_state.current_progress = None;
                    refresh_needed = true;

                    if context.config.settings.transfer_auto_report {
                        let format = &context.config.settings.transfer_report_format;
                        let content = if format == "csv" {
                            crate::fs::transfer::report::generate_csv_report(&results)
                        } else {
                            crate::fs::transfer::report::generate_html_report(&results, &format!("Job {}", job_id))
                        };
                        let dest_dir = if let Some(first_file) = results.completed_files.first() {
                            first_file.dst.parent().unwrap_or(std::path::Path::new("."))
                        } else {
                            std::path::Path::new(".")
                        };
                        if let Ok(report_path) = crate::fs::transfer::report::save_report(&content, format, dest_dir) {
                            transfer_state.log_lines.push(format!("Saved report to: {}", report_path.to_string_lossy()));
                        }
                    }

                    // Ejecutar post action si la cola se ha vaciado
                    if transfer_state.engine.queue.pending_count() == 0 && transfer_state.post_action != crate::fs::transfer::post_action::PostAction::None {
                        transfer_state.log_lines.push(format!("Executing post-action: {:?}", transfer_state.post_action));
                        let _ = crate::fs::transfer::post_action::execute_post_action(transfer_state.post_action.clone());
                    }
                }
                TransferEvent::JobFailed { error, job_id } => {
                    transfer_state.log_lines.push(format!("[{}] Job failed: {}", job_id, error));
                    transfer_state.current_progress = None;
                    refresh_needed = true;
                }
                TransferEvent::ConflictDetected { job_id, file, conflict } => {
                    transfer_state.log_lines.push(format!("Conflict detected: {}", file.to_string_lossy()));
                    transfer_state.active_conflict_info = Some((job_id, file, conflict));
                    transfer_state.view_mode = crate::app::state::TransferViewMode::Expanded;
                    state.active_popup = Some(crate::app::state::types::PopupType::TransferPanel);
                    refresh_needed = true;
                }
                TransferEvent::VerifyStarted { job_id, file, algorithm } => {
                    let _ = job_id;
                    transfer_state.log_lines.push(format!("🔍 Verifying [{}]: {}", algorithm, file.to_string_lossy()));
                }
                TransferEvent::VerifyProgress { job_id, bytes_verified, bytes_total } => {
                    let _ = job_id;
                    if let Some(ref mut prog) = transfer_state.current_progress {
                        prog.bytes_transferred = bytes_verified;
                        prog.bytes_total = prog.bytes_total.max(bytes_total);
                    }
                }
            }
        }

        if transfer_state.log_lines.len() > 1000 {
            let drain_count = transfer_state.log_lines.len() - 1000;
            transfer_state.log_lines.drain(0..drain_count);
        }
    }
    if refresh_needed {
        state.refresh_both_panels(context.config.settings.show_hidden);
    }

    if let Some(cmd) = state.pending_custom_command.take() {
        let active_path = state.get_active_panel().current_path.clone();
        let _ = crate::app::actions::exec::execute_shell_command(
            &cmd,
            &active_path,
            context,
            terminal_backend,
        );
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
}
