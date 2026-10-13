//! Translated texts shared by the transfer bar, panel and conflict dialog:
//! job status, operation, post-action, yes / no and ETA.

use crate::config::localization::t;
use crate::fs::transfer::job::{TransferJob, TransferJobStatus, TransferOperation};
use crate::fs::transfer::post_action::PostAction;

/// "Queued", "Running (45%)", "Completed"...
pub fn status(job: &TransferJob) -> String {
    let key = match job.status {
        TransferJobStatus::Queued => "transfer_status_queued",
        TransferJobStatus::Scanning => "transfer_status_scanning",
        TransferJobStatus::Transferring => {
            let percent = job.progress.as_ref().map_or(0.0, |p| p.percent_bytes());
            return t("transfer_status_running").replace("{}", &format!("{percent:.0}"));
        }
        TransferJobStatus::Verifying => "transfer_status_verifying",
        TransferJobStatus::Paused => "transfer_status_paused",
        TransferJobStatus::Completed => "transfer_status_completed",
        TransferJobStatus::Failed => "transfer_status_failed",
        TransferJobStatus::Cancelled => "transfer_status_cancelled",
    };
    t(key)
}

/// "Copy", "Move"...
pub fn operation(op: TransferOperation) -> String {
    t(match op {
        TransferOperation::Copy => "transfer_op_copy",
        TransferOperation::Move => "transfer_op_move",
        TransferOperation::Delete => "transfer_op_delete",
        TransferOperation::Wipe => "transfer_op_wipe",
        TransferOperation::Compress => "transfer_op_compress",
        TransferOperation::Extract => "transfer_op_extract",
        TransferOperation::ApplyCommand => "transfer_op_apply",
        TransferOperation::Restore => "transfer_op_restore",
    })
}

/// What runs when the queue finishes.
pub fn post_action(action: &PostAction) -> String {
    match action {
        PostAction::None => t("transfer_post_none"),
        PostAction::Shutdown => t("transfer_post_shutdown"),
        PostAction::Sleep => t("transfer_post_sleep"),
        PostAction::Hibernate => t("transfer_post_hibernate"),
        PostAction::EjectDrive(drive) => t("transfer_post_eject").replace("{}", drive),
        PostAction::RunScript(script) => {
            t("transfer_post_script").replace("{}", &script.to_string_lossy())
        }
        PostAction::CloseApp => t("transfer_post_close"),
    }
}

pub fn yes_no(value: bool) -> String {
    t(if value { "info_yes" } else { "info_no" })
}

/// "ETA 12s", or "ETA --" when unknown.
pub fn eta(seconds: Option<u64>) -> String {
    match seconds {
        Some(secs) => t("transfer_eta").replace("{}", &secs.to_string()),
        None => t("transfer_eta_unknown"),
    }
}
