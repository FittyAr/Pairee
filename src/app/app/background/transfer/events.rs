use crate::app::context::AppContext;
use crate::app::state::popup::{ErrorMessage, TransferPanel};
use crate::app::state::types::PopupType;
use crate::app::state::{AppState, TransferUIState};
use crate::config::localization::t;
use crate::fs::transfer::events::TransferEvent;
use crate::fs::transfer::job::TransferResults;
use uuid::Uuid;

/// UI follow-ups collected while draining transfer events.
#[derive(Default)]
pub struct EventEffects {
    /// Output lines (`Some`) or end of output (`None`) for terminal screens.
    pub term_forwards: Vec<(Uuid, Option<String>)>,
    pub refresh_needed: bool,
    /// Jobs that ended, with the final results of the completed ones.
    pub finished: Vec<(Uuid, Option<TransferResults>)>,
    /// The queue finished with "close the application" as post action.
    pub quit: bool,
}

pub fn handle_transfer_event(
    event: TransferEvent,
    state: &mut AppState,
    context: &AppContext,
    effects: &mut EventEffects,
) {
    let Some(transfer) = state.transfer.as_mut() else {
        return;
    };
    let job_id = event.job_id();
    transfer.engine.queue.update_job(job_id, |job| {
        crate::fs::transfer::job_log::apply_event(job, &event)
    });

    match event {
        TransferEvent::CommandOutput { line, .. } => {
            effects.term_forwards.push((job_id, Some(line)))
        }
        TransferEvent::SpeedUpdate {
            bytes_per_second,
            eta_seconds,
            ..
        } => transfer.speed_info = (bytes_per_second, eta_seconds),
        TransferEvent::JobCompleted { results, .. } => {
            effects.term_forwards.push((job_id, None));
            effects.refresh_needed = true;
            if context.config.settings.transfer.auto_report {
                save_report(transfer, context, job_id, &results);
            }
            if let Some(first) = results.failed_files.first() {
                let message = t("transfer_files_failed")
                    .replacen("{}", &results.failed_files.len().to_string(), 1)
                    .replacen("{}", &first.src.to_string_lossy(), 1)
                    .replacen("{}", &first.error, 1);
                show_failure(state, message);
            }
            let Some(transfer) = state.transfer.as_mut() else {
                return;
            };
            // Where the job wrote: the drive an "eject" post-action ejects.
            let destination = results.completed_files.last().map(|f| f.dst.clone());
            effects.finished.push((job_id, Some(results)));
            use crate::fs::transfer::post_action::{PostAction, execute_post_action};
            if transfer.engine.queue.pending_count() == 0 {
                match transfer.post_action.clone() {
                    PostAction::None => {}
                    // Quit through the event loop (terminal restored, session
                    // saved, unsaved editors kept), never `process::exit`.
                    PostAction::CloseApp => effects.quit = true,
                    action => {
                        let _ = execute_post_action(action, destination.as_deref());
                    }
                }
            }
        }
        TransferEvent::JobFailed { error, .. } => {
            effects.finished.push((job_id, None));
            effects.term_forwards.push((job_id, None));
            effects.refresh_needed = true;
            // The panel bar is gone once nothing runs: without this the
            // job would end without a word.
            if error != t("transfer_job_cancelled") {
                show_failure(state, error);
            }
        }
        TransferEvent::ConflictDetected { file, conflict, .. } => {
            transfer.active_conflict_info = Some((job_id, file, conflict));
            transfer.view_mode = crate::app::state::TransferViewMode::Expanded;
            state
                .dialogs
                .replace(crate::app::state::types::PopupType::TransferPanel(
                    TransferPanel,
                ));
            effects.refresh_needed = true;
        }
        _ => {}
    }
}

/// Shows why a job failed, once (a failure can be reported twice).
fn show_failure(state: &mut AppState, message: String) {
    let shown = state
        .dialogs
        .iter()
        .any(|d| matches!(d, PopupType::Error(ErrorMessage(text)) if *text == message));
    if !shown {
        state.dialogs.replace(PopupType::error(message));
    }
}

/// Writes the CSV / HTML report next to the first copied file.
fn save_report(
    transfer: &TransferUIState,
    context: &AppContext,
    job_id: Uuid,
    results: &TransferResults,
) {
    let format = &context.config.settings.transfer.report_format;
    let content = if format == "csv" {
        crate::fs::transfer::report::generate_csv_report(results)
    } else {
        crate::fs::transfer::report::generate_html_report(results, &format!("Job {}", job_id))
    };
    let dest_dir = results
        .completed_files
        .first()
        .and_then(|f| f.dst.parent())
        .unwrap_or(std::path::Path::new("."));
    if let Ok(report_path) = crate::fs::transfer::report::save_report(&content, format, dest_dir) {
        transfer.engine.queue.update_job(job_id, |job| {
            job.log_lines.push(format!(
                "Saved report to: {}",
                report_path.to_string_lossy()
            ));
        });
    }
}
