use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferUIState};
use crate::fs::transfer::events::TransferEvent;
use crate::fs::transfer::job::TransferResults;
use uuid::Uuid;

/// UI follow-ups collected while draining transfer events.
#[derive(Default)]
pub struct EventEffects {
    /// Output lines (`Some`) or end of output (`None`) for terminal screens.
    pub term_forwards: Vec<(Uuid, Option<String>)>,
    pub refresh_needed: bool,
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
            if context.config.settings.transfer_auto_report {
                save_report(transfer, context, job_id, &results);
            }
            if transfer.engine.queue.pending_count() == 0
                && transfer.post_action != crate::fs::transfer::post_action::PostAction::None
            {
                let _ = crate::fs::transfer::post_action::execute_post_action(
                    transfer.post_action.clone(),
                );
            }
        }
        TransferEvent::JobFailed { .. } => {
            effects.term_forwards.push((job_id, None));
            effects.refresh_needed = true;
        }
        TransferEvent::ConflictDetected { file, conflict, .. } => {
            transfer.active_conflict_info = Some((job_id, file, conflict));
            transfer.view_mode = crate::app::state::TransferViewMode::Expanded;
            state
                .dialogs
                .replace(crate::app::state::types::PopupType::TransferPanel);
            effects.refresh_needed = true;
        }
        _ => {}
    }
}

/// Writes the CSV / HTML report next to the first copied file.
fn save_report(
    transfer: &TransferUIState,
    context: &AppContext,
    job_id: Uuid,
    results: &TransferResults,
) {
    let format = &context.config.settings.transfer_report_format;
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
