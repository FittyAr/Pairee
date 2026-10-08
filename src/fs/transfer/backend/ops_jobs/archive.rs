//! Archival operation runner (compress, extract) with cooperative cancellation.

use super::super::super::events::TransferEvent;
use super::super::super::job::{SkippedFile, TransferResults};
use crate::config::localization::t;
use crate::fs::progress::ProgressUpdate;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::control::failed;
use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

pub async fn run_compress(
    sources: Vec<PathBuf>,
    dest_archive: PathBuf,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    run_archive_blocking(
        control,
        sources.len().max(1),
        move |tx, cancel| crate::fs::archive::compress_zip(sources, &dest_archive, tx, cancel),
        "error_compression_failed",
    )
    .await
}

pub async fn run_extract(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let archive = sources
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Extract requires an archive path in sources"))?;
    let dest = destination_dir.clone();
    let archive_for_complete = archive.clone();

    let mut results = run_archive_blocking(
        control,
        1,
        move |tx, cancel| crate::fs::archive::extract_archive(&archive, &dest, tx, cancel),
        "error_extraction_failed",
    )
    .await?;

    if results.completed_files.is_empty() && results.failed_files.is_empty() {
        results
            .completed_files
            .push(super::super::super::job::FileTransferResult {
                src: archive_for_complete,
                dst: destination_dir,
                size: 0,
                src_hash: None,
                dst_hash: None,
                verified: true,
                duration: std::time::Duration::ZERO,
            });
    }
    Ok(results)
}

async fn run_archive_blocking<F>(
    control: JobControl,
    default_total: usize,
    work: F,
    err_key: &str,
) -> Result<TransferResults, anyhow::Error>
where
    F: FnOnce(&mpsc::Sender<ProgressUpdate>, &AtomicBool) -> anyhow::Result<()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<ProgressUpdate>(64);
    let cancel = Arc::clone(&control.is_cancelled);

    let work_handle = tokio::task::spawn_blocking(move || work(&tx, cancel.as_ref()));

    let results = bridge_progress_to_events(rx, &control, default_total).await?;

    match work_handle.await {
        Ok(Ok(())) => {
            if control.is_cancelled() {
                return Err(anyhow!("Job cancelled"));
            }
            control.job_completed(results)
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.to_lowercase().contains("cancel") || control.is_cancelled() {
                return Err(anyhow!("Job cancelled"));
            }
            let err_msg = t(err_key).replacen("{}", &msg, 1);
            let _ = control.event_tx.send(TransferEvent::JobFailed {
                job_id: control.job_id,
                error: err_msg.clone(),
            });
            Err(anyhow!(err_msg))
        }
        Err(e) => Err(anyhow!("archive task join error: {e}")),
    }
}

async fn bridge_progress_to_events(
    mut rx: mpsc::Receiver<ProgressUpdate>,
    control: &JobControl,
    default_total: usize,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let mut announced_scan = false;

    while let Some(update) = rx.recv().await {
        if control.is_cancelled() {
            while rx.try_recv().is_ok() {}
            break;
        }

        let total = if update.total_files > 0 {
            update.total_files
        } else {
            default_total
        };

        if !announced_scan {
            announced_scan = true;
            control.scan_complete(total, update.total_bytes);
        }

        if update.skipped {
            let file = PathBuf::from(&update.current_file);
            let reason = t("archive_entry_skipped_exists");
            results.skipped_files.push(SkippedFile {
                src: file.clone(),
                reason: reason.clone(),
            });
            let _ = control.event_tx.send(TransferEvent::FileSkipped {
                job_id: control.job_id,
                file,
                reason,
            });
            continue;
        }

        if let Some(err) = update.error {
            let path = PathBuf::from(&update.current_file);
            control.file_failed(&mut results, failed(&path, PathBuf::new(), err.clone()));
            return Err(anyhow!(err));
        }

        if update.current_file == "Completed" {
            continue;
        }

        let path = PathBuf::from(&update.current_file);
        control.file_started(&path, update.files_copied);
        if update.total_bytes > 0 {
            let _ = control.event_tx.send(TransferEvent::FileProgress {
                job_id: control.job_id,
                bytes_copied: update.bytes_copied,
                bytes_total: update.total_bytes,
            });
        }

        let result = super::super::super::job::FileTransferResult {
            src: path.clone(),
            dst: path,
            size: 0,
            src_hash: None,
            dst_hash: None,
            verified: true,
            duration: std::time::Duration::ZERO,
        };
        control.file_completed(&mut results, result);
    }

    Ok(results)
}
