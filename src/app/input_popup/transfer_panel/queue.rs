//! Queue management actions for transfer panel (pause, cancel, retry, export, reorder).

use crate::app::context::AppContext;
use crate::app::state::popup::ConfirmInterrupt;
use crate::app::state::{DialogStack, TransferUIState};
use crate::fs::transfer::job::{TransferJob, TransferJobStatus};
use crossterm::event::KeyCode;
use std::sync::atomic::Ordering;

/// Queue command bound to a key of the transfer panel.
#[derive(Clone, Copy)]
enum QueueOp {
    TogglePause,
    SkipFile,
    Cancel,
    RetryFailed,
    ExportReport,
    MoveUp,
    MoveDown,
    ClearCompleted,
    Remove,
}

impl QueueOp {
    fn from_key(code: KeyCode) -> Option<Self> {
        Some(match code {
            KeyCode::Char('p' | 'P') => Self::TogglePause,
            KeyCode::Char('s' | 'S') => Self::SkipFile,
            KeyCode::Char('x' | 'X') => Self::Cancel,
            KeyCode::Char('r' | 'R') => Self::RetryFailed,
            KeyCode::Char('e' | 'E') => Self::ExportReport,
            KeyCode::Char('+' | '=') => Self::MoveUp,
            KeyCode::Char('-') => Self::MoveDown,
            KeyCode::Char('c' | 'C') => Self::ClearCompleted,
            KeyCode::Delete => Self::Remove,
            _ => return None,
        })
    }
}

pub fn handle_queue_action(
    transfer: &mut TransferUIState,
    dialogs: &mut DialogStack,
    context: &AppContext,
    code: KeyCode,
) -> bool {
    let Some(op) = QueueOp::from_key(code) else {
        return false;
    };
    match op {
        QueueOp::ClearCompleted => {
            transfer.engine.queue.clear_completed();
            transfer.queue_cursor = 0;
            return true;
        }
        QueueOp::Cancel
            if context
                .config
                .settings
                .confirmations
                .confirm_interrupt_operation =>
        {
            // The panel overlay is drawn above every dialog: minimize it
            // while the question is open (its answer reopens the panel).
            transfer.view_mode = crate::app::state::TransferViewMode::Minimized;
            dialogs.replace(ConfirmInterrupt);
            return true;
        }
        _ => {}
    }
    let jobs = transfer.engine.queue.get_all();
    if let Some(job) = jobs.get(transfer.queue_cursor) {
        apply_to_job(transfer, context, op, &jobs, job);
    }
    true
}

/// Runs `op` on `job`, the queue entry under the cursor.
fn apply_to_job(
    transfer: &mut TransferUIState,
    context: &AppContext,
    op: QueueOp,
    jobs: &[TransferJob],
    job: &TransferJob,
) {
    let queue = &transfer.engine.queue;
    match op {
        QueueOp::TogglePause => {
            toggle_pause(transfer, jobs, job);
            // A queued job moves ahead of the others: keep it selected.
            if let Some(idx) = queue.get_all().iter().position(|j| j.id == job.id) {
                transfer.queue_cursor = idx;
            }
        }
        QueueOp::SkipFile => job.skip_file_flag.store(true, Ordering::SeqCst),
        QueueOp::Cancel => cancel_job(transfer, job),
        QueueOp::RetryFailed => retry_failed(transfer, job),
        QueueOp::ExportReport => export_report(transfer, context, job),
        QueueOp::MoveUp => {
            if queue.reorder(job.id, -1) {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            }
        }
        QueueOp::MoveDown => {
            if queue.reorder(job.id, 1) && transfer.queue_cursor < jobs.len().saturating_sub(1) {
                transfer.queue_cursor += 1;
            }
        }
        QueueOp::Remove => {
            if queue.remove(job.id) {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            }
        }
        QueueOp::ClearCompleted => {}
    }
}

/// Cancels `job`; a finished one keeps its outcome (it is not relabelled).
pub fn cancel_job(transfer: &TransferUIState, job: &TransferJob) {
    if job.is_terminal() {
        return;
    }
    job.cancel();
    transfer
        .engine
        .queue
        .update_job(job.id, |j| j.status = TransferJobStatus::Cancelled);
}

/// Scanning, transferring or verifying (not paused).
fn is_running(status: &TransferJobStatus) -> bool {
    matches!(
        status,
        TransferJobStatus::Transferring
            | TransferJobStatus::Scanning
            | TransferJobStatus::Verifying
    )
}

/// Resuming a paused job pauses the running ones; a running job is
/// paused; a queued job moves to the front of the queue, to run next. The
/// engine runs one job at a time: pausing the running job for a queued
/// one would only stall both.
fn toggle_pause(transfer: &TransferUIState, jobs: &[TransferJob], job: &TransferJob) {
    let queue = &transfer.engine.queue;
    match job.status {
        TransferJobStatus::Paused => {
            pause_running(transfer, jobs, job);
            job.is_paused.store(false, Ordering::SeqCst);
            queue.update_job(job.id, |j| j.status = TransferJobStatus::Transferring);
        }
        ref status if is_running(status) => {
            job.is_paused.store(true, Ordering::SeqCst);
            queue.update_job(job.id, |j| j.status = TransferJobStatus::Paused);
        }
        TransferJobStatus::Queued => {
            job.is_paused.store(false, Ordering::SeqCst);
            queue.move_to_front(job.id);
        }
        _ => {}
    }
}

/// Pauses the jobs other than `job` that are running (queued ones keep
/// their turn and start unpaused).
fn pause_running(transfer: &TransferUIState, jobs: &[TransferJob], job: &TransferJob) {
    for other_job in jobs
        .iter()
        .filter(|other| other.id != job.id && is_running(&other.status))
    {
        other_job.is_paused.store(true, Ordering::SeqCst);
        transfer
            .engine
            .queue
            .update_job(other_job.id, |j| j.status = TransferJobStatus::Paused);
    }
}

/// Queues a new job with the files that failed in `job`.
fn retry_failed(transfer: &mut TransferUIState, job: &TransferJob) {
    let Some(new_job) = job.retry_failed() else {
        return;
    };
    let failed = job.results.failed_files.len();
    transfer.engine.queue.update_job(job.id, |j| {
        j.log_lines
            .push(format!("Re-enqueueing {failed} failed files..."));
    });
    transfer.engine.submit_job(new_job);
}

/// Saves the job report (CSV or HTML) next to the first copied file.
fn export_report(transfer: &TransferUIState, context: &AppContext, job: &TransferJob) {
    use crate::fs::transfer::report;
    let res = &job.results;
    let format = context.config.settings.transfer.report_format.clone();
    let content = if format == "csv" {
        report::generate_csv_report(res)
    } else {
        report::generate_html_report(res, "Manual Export")
    };
    let dest_dir = res
        .completed_files
        .first()
        .and_then(|first_file| first_file.dst.parent())
        .unwrap_or(std::path::Path::new("."));
    if let Ok(report_path) = report::save_report(&content, &format, dest_dir) {
        transfer.engine.queue.update_job(job.id, |j| {
            j.log_lines.push(format!(
                "Manually saved report to: {}",
                report_path.to_string_lossy()
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::TransferPanel;
    use crate::app::state::{AppState, PopupType, TransferViewMode};
    use crate::config::AppConfig;
    use crate::fs::transfer::job::TransferOperation;

    #[tokio::test]
    async fn cancel_keeps_finished_jobs_and_asks_above_the_panel() {
        let mut state = AppState::new(".".into(), ".".into());
        let mut context = AppContext::new(AppConfig::default());
        crate::app::transfer_submit::ensure_transfer_ui(&mut state);
        let ts = state.transfer.as_mut().unwrap();
        let mut job = TransferJob::new(
            TransferOperation::Copy,
            Vec::new(),
            Default::default(),
            Default::default(),
        );
        job.status = TransferJobStatus::Completed;
        let id = job.id;
        ts.engine.queue.enqueue(job);
        ts.view_mode = TransferViewMode::Expanded;
        state.dialogs.replace(TransferPanel);

        context
            .config
            .settings
            .confirmations
            .confirm_interrupt_operation = false;
        let ts = state.transfer.as_mut().unwrap();
        assert!(handle_queue_action(
            ts,
            &mut state.dialogs,
            &context,
            KeyCode::Char('x')
        ));
        let status = ts.engine.queue.get(id).unwrap().status;
        assert_eq!(status, TransferJobStatus::Completed, "not relabelled");

        context
            .config
            .settings
            .confirmations
            .confirm_interrupt_operation = true;
        handle_queue_action(ts, &mut state.dialogs, &context, KeyCode::Char('x'));
        assert_eq!(
            ts.view_mode,
            TransferViewMode::Minimized,
            "question visible"
        );
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::ConfirmInterrupt(_))
        ));
        let esc = crossterm::event::KeyEvent::from(KeyCode::Esc);
        crate::app::input_popup::handle_popup_input(&mut state, esc, &mut context).unwrap();
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::TransferPanel(_))
        ));
        let ts = state.transfer.as_ref().unwrap();
        assert_eq!(
            ts.view_mode,
            TransferViewMode::Expanded,
            "back to the panel"
        );
    }

    /// P on a queued job makes it run next without pausing the running
    /// one (the engine runs one job at a time).
    #[tokio::test]
    async fn starting_a_queued_job_does_not_pause_the_running_one() {
        let mut state = AppState::new(".".into(), ".".into());
        crate::app::transfer_submit::ensure_transfer_ui(&mut state);
        let ts = state.transfer.as_mut().unwrap();
        let job = |status| {
            let mut job = TransferJob::new(
                TransferOperation::Copy,
                Vec::new(),
                Default::default(),
                Default::default(),
            );
            job.status = status;
            job
        };
        let (running, waiting, queued) = (
            job(TransferJobStatus::Transferring),
            job(TransferJobStatus::Queued),
            job(TransferJobStatus::Queued),
        );
        let ids = (running.id, waiting.id, queued.id);
        for j in [running, waiting, queued] {
            ts.engine.queue.enqueue(j);
        }
        let jobs = ts.engine.queue.get_all();
        toggle_pause(ts, &jobs, &jobs[2]);
        let all = ts.engine.queue.get_all();
        let get = |id| all.iter().find(|j| j.id == id).unwrap();
        assert_eq!(get(ids.0).status, TransferJobStatus::Transferring);
        assert!(!get(ids.0).is_paused.load(Ordering::SeqCst));
        assert!(!get(ids.1).is_paused.load(Ordering::SeqCst));
        let first_queued = all
            .iter()
            .find(|j| j.status == TransferJobStatus::Queued)
            .unwrap();
        assert_eq!(first_queued.id, ids.2, "runs next");
    }
}
