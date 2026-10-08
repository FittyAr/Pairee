//! Applies [`TransferEvent`]s to the job they belong to: progress counters,
//! result lists and the human-readable job log. Pure bookkeeping (no UI).

use super::events::TransferEvent;
use super::job::{
    SkippedFile, TransferJob, TransferJobStatus, TransferOperation, TransferProgress,
    TransferResults,
};

/// Log texts that depend on the operation.
struct OperationLog {
    scanning: &'static str,
    /// Verb shown for each file ("Copying"...).
    verb: &'static str,
    /// Past tense shown when a file is done (`None`: show the destination).
    done: Option<&'static str>,
}

fn operation_log(op: TransferOperation) -> OperationLog {
    let (scanning, verb, done) = match op {
        TransferOperation::Copy => ("Scanning source files...", "Copying", None),
        TransferOperation::Move => ("Scanning source files...", "Moving", None),
        TransferOperation::Delete => (
            "Scanning source files for deletion...",
            "Deleting",
            Some("Deleted"),
        ),
        TransferOperation::Wipe => (
            "Scanning source files for secure wipe...",
            "Wiping",
            Some("Wiped"),
        ),
        TransferOperation::Compress => (
            "Preparing files for compression...",
            "Compressing",
            Some("Packed"),
        ),
        TransferOperation::Extract => (
            "Preparing archive for extraction...",
            "Extracting",
            Some("Extracted"),
        ),
        TransferOperation::ApplyCommand => (
            "Preparing apply-command targets...",
            "Applying",
            Some("Applied"),
        ),
    };
    OperationLog {
        scanning,
        verb,
        done,
    }
}

/// Updates `job` for `event` (events of other jobs must not be passed).
pub fn apply_event(job: &mut TransferJob, event: &TransferEvent) {
    let log = operation_log(job.operation);
    match event {
        TransferEvent::JobStarted { job_id } => {
            job.progress = Some(TransferProgress::default());
            job.results = TransferResults::default();
            job.log_lines.push(format!("[{}] Job started", job_id));
        }
        TransferEvent::ScanStarted { .. } => {
            job.status = TransferJobStatus::Scanning;
            job.progress = Some(TransferProgress::default());
            job.log_lines.push(log.scanning.to_string());
        }
        TransferEvent::ScanProgress { files_found, .. } => {
            progress(job, |p| p.files_scanned = *files_found);
        }
        TransferEvent::ScanComplete {
            total_files,
            total_bytes,
            ..
        } => {
            progress(job, |p| {
                p.files_total = *total_files;
                p.bytes_total = *total_bytes;
            });
            job.log_lines.push(format!(
                "Scan complete: {} files, {}",
                total_files,
                bytesize::ByteSize(*total_bytes)
            ));
        }
        TransferEvent::FileStarted { file, index, .. } => {
            let file = file.to_string_lossy();
            progress(job, |p| p.current_file = file.to_string());
            job.log_lines
                .push(format!("[{}] {}: {}", index + 1, log.verb, file));
        }
        TransferEvent::FileProgress {
            bytes_copied: done,
            bytes_total: total,
            ..
        }
        | TransferEvent::VerifyProgress {
            bytes_verified: done,
            bytes_total: total,
            ..
        } => progress(job, |p| {
            p.bytes_transferred = *done;
            p.bytes_total = p.bytes_total.max(*total);
        }),
        TransferEvent::FileCompleted { result, .. } => {
            progress(job, |p| p.files_completed += 1);
            job.results.completed_files.push(result.clone());
            job.log_lines.push(match log.done {
                Some(done) => format!("✓ OK: {} {}", done, result.src.to_string_lossy()),
                None => {
                    let hash = if result.verified { " ✓hash" } else { "" };
                    format!("✓ OK{}: {}", hash, result.dst.to_string_lossy())
                }
            });
        }
        TransferEvent::FileFailed { error, .. } => {
            progress(job, |p| p.files_failed += 1);
            job.results.failed_files.push(error.clone());
            job.log_lines.push(format!(
                "✗ FAIL: {} - {}",
                error.src.to_string_lossy(),
                error.error
            ));
        }
        TransferEvent::FileSkipped { file, reason, .. } => {
            progress(job, |p| p.files_skipped += 1);
            job.results.skipped_files.push(SkippedFile {
                src: file.clone(),
                reason: reason.clone(),
            });
            job.log_lines
                .push(format!("⚠ SKIP: {} - {}", file.to_string_lossy(), reason));
        }
        TransferEvent::SpeedUpdate {
            bytes_per_second,
            eta_seconds,
            ..
        } => progress(job, |p| {
            p.bytes_per_second = *bytes_per_second;
            p.eta_seconds = *eta_seconds;
        }),
        TransferEvent::JobCompleted { job_id, .. } => {
            job.log_lines
                .push(format!("[{}] Job completed successfully", job_id));
        }
        TransferEvent::JobFailed { job_id, error } => {
            job.log_lines
                .push(format!("[{}] Job failed: {}", job_id, error));
        }
        TransferEvent::ConflictDetected { file, .. } => {
            job.log_lines
                .push(format!("Conflict detected: {}", file.to_string_lossy()));
        }
        TransferEvent::VerifyStarted {
            file, algorithm, ..
        } => {
            job.log_lines.push(format!(
                "🔍 Verifying [{}]: {}",
                algorithm,
                file.to_string_lossy()
            ));
        }
        TransferEvent::CommandOutput { .. } => {}
    }
}

/// Runs `update` on the job progress, if the job has started.
fn progress(job: &mut TransferJob, update: impl FnOnce(&mut TransferProgress)) {
    if let Some(p) = job.progress.as_mut() {
        update(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn logs_use_the_operation_verb() {
        let mut job = TransferJob::new(
            TransferOperation::Delete,
            vec![],
            PathBuf::new(),
            Default::default(),
        );
        let job_id = Uuid::new_v4();
        apply_event(&mut job, &TransferEvent::JobStarted { job_id });
        apply_event(
            &mut job,
            &TransferEvent::FileStarted {
                job_id,
                file: PathBuf::from("a"),
                index: 0,
            },
        );
        apply_event(
            &mut job,
            &TransferEvent::FileSkipped {
                job_id,
                file: PathBuf::from("b"),
                reason: "r".into(),
            },
        );
        assert_eq!(job.log_lines[1], "[1] Deleting: a");
        assert_eq!(job.progress.as_ref().unwrap().current_file, "a");
        assert_eq!(job.progress.as_ref().unwrap().files_skipped, 1);
        assert_eq!(job.results.skipped_files.len(), 1);
    }
}
