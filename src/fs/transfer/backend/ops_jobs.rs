//! Wipe / Compress / Extract backends for the Transfer Engine.
//!
//! These are not classic copy/move transfers, but they share the same job
//! queue and [`TransferEvent`] UI so users keep one interaction model.
//!
//! Pattern: **Strategy** (per-op runner) + **Adapter** (ProgressUpdate → events
//! for existing `archive` helpers).

use super::super::events::TransferEvent;
use super::super::job::{FailedFile, FileTransferResult, TransferOperation, TransferResults};
use super::BackendControl;
use crate::config::localization::t;
use crate::fs::ops_worker::ProgressUpdate;
use anyhow::anyhow;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

pub async fn run_ops_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    match operation {
        TransferOperation::Wipe => run_wipe(sources, control).await,
        TransferOperation::Compress => run_compress(sources, destination, control).await,
        TransferOperation::Extract => run_extract(sources, destination, control).await,
        other => Err(anyhow!("ops backend does not handle {:?}", other.label())),
    }
}

async fn run_wipe(
    sources: Vec<PathBuf>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let total = sources.len();
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files: total,
        total_bytes: 0,
    });

    let mut results = TransferResults::default();

    for (idx, path) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        control.wait_if_paused();
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: path.clone(),
            index: idx,
        });

        // wipe_file is sync/blocking; run off the UI reactor.
        let wipe_path = path.clone();
        let wipe_res = tokio::task::spawn_blocking(move || crate::fs::wipe::wipe_file(&wipe_path))
            .await
            .map_err(|e| anyhow!("wipe task join error: {e}"))?;

        match wipe_res {
            Ok(()) => {
                let result = FileTransferResult {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                };
                results.completed_files.push(result.clone());
                let _ = control.event_tx.send(TransferEvent::FileCompleted {
                    job_id: control.job_id,
                    result,
                });
            }
            Err(e) => {
                let err_msg = t("error_wipe_failed_for")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                let failed = FailedFile {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    error: err_msg.clone(),
                    retries: 0,
                };
                results.failed_files.push(failed.clone());
                let _ = control.event_tx.send(TransferEvent::FileFailed {
                    job_id: control.job_id,
                    error: failed,
                });
                let _ = control.event_tx.send(TransferEvent::JobFailed {
                    job_id: control.job_id,
                    error: err_msg.clone(),
                });
                return Err(anyhow!(err_msg));
            }
        }
    }

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}

async fn run_compress(
    sources: Vec<PathBuf>,
    dest_archive: PathBuf,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let (tx, rx) = mpsc::channel::<ProgressUpdate>(64);
    let dest = dest_archive.clone();
    let sources_clone = sources.clone();

    let work = tokio::task::spawn_blocking(move || {
        crate::fs::archive::compress_zip(sources_clone, &dest, &tx)
    });

    let results = bridge_progress_to_events(rx, &control, sources.len().max(1)).await?;

    match work.await {
        Ok(Ok(())) => {
            let _ = control.event_tx.send(TransferEvent::JobCompleted {
                job_id: control.job_id,
                results: results.clone(),
            });
            Ok(results)
        }
        Ok(Err(e)) => {
            let err_msg = t("error_compression_failed").replacen("{}", &e.to_string(), 1);
            let _ = control.event_tx.send(TransferEvent::JobFailed {
                job_id: control.job_id,
                error: err_msg.clone(),
            });
            Err(anyhow!(err_msg))
        }
        Err(e) => Err(anyhow!("compress task join error: {e}")),
    }
}

async fn run_extract(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let archive = sources
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Extract requires an archive path in sources"))?;

    let (tx, rx) = mpsc::channel::<ProgressUpdate>(64);
    let archive_for_work = archive.clone();
    let dest = destination_dir.clone();

    let work = tokio::task::spawn_blocking(move || {
        crate::fs::archive::extract_archive(&archive_for_work, &dest, &tx)
    });

    let results = bridge_progress_to_events(rx, &control, 1).await?;

    match work.await {
        Ok(Ok(())) => {
            // Ensure at least one completed entry for the archive itself.
            let mut results = results;
            if results.completed_files.is_empty() {
                results.completed_files.push(FileTransferResult {
                    src: archive,
                    dst: destination_dir,
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: std::time::Duration::ZERO,
                });
            }
            let _ = control.event_tx.send(TransferEvent::JobCompleted {
                job_id: control.job_id,
                results: results.clone(),
            });
            Ok(results)
        }
        Ok(Err(e)) => {
            let err_msg = t("error_extraction_failed").replacen("{}", &e.to_string(), 1);
            let _ = control.event_tx.send(TransferEvent::JobFailed {
                job_id: control.job_id,
                error: err_msg.clone(),
            });
            Err(anyhow!(err_msg))
        }
        Err(e) => Err(anyhow!("extract task join error: {e}")),
    }
}

/// Adapter: map legacy [`ProgressUpdate`] stream into [`TransferEvent`]s.
async fn bridge_progress_to_events(
    mut rx: mpsc::Receiver<ProgressUpdate>,
    control: &BackendControl,
    default_total: usize,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let mut announced_scan = false;

    while let Some(update) = rx.recv().await {
        if control.cancelled() {
            // Drop remaining progress; join will still finish the blocking work.
            // Ideal cancel would abort archive mid-write; not supported yet.
            break;
        }

        let total = if update.total_files > 0 {
            update.total_files
        } else {
            default_total
        };

        if !announced_scan {
            announced_scan = true;
            let _ = control.event_tx.send(TransferEvent::ScanComplete {
                job_id: control.job_id,
                total_files: total,
                total_bytes: update.total_bytes,
            });
        }

        if let Some(err) = update.error {
            let path = PathBuf::from(&update.current_file);
            let failed = FailedFile {
                src: path.clone(),
                dst: PathBuf::new(),
                error: err.clone(),
                retries: 0,
            };
            results.failed_files.push(failed.clone());
            let _ = control.event_tx.send(TransferEvent::FileFailed {
                job_id: control.job_id,
                error: failed,
            });
            return Err(anyhow!(err));
        }

        if update.current_file == "Completed" {
            continue;
        }

        let index = update.files_copied;
        let path = PathBuf::from(&update.current_file);
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: path.clone(),
            index,
        });
        if update.total_bytes > 0 {
            let _ = control.event_tx.send(TransferEvent::FileProgress {
                job_id: control.job_id,
                bytes_copied: update.bytes_copied,
                bytes_total: update.total_bytes,
            });
        }

        // compress_zip / extract emit one progress tick per entry; treat as completed.
        let result = FileTransferResult {
            src: path.clone(),
            dst: path,
            size: 0,
            src_hash: None,
            dst_hash: None,
            verified: true,
            duration: std::time::Duration::ZERO,
        };
        results.completed_files.push(result.clone());
        let _ = control.event_tx.send(TransferEvent::FileCompleted {
            job_id: control.job_id,
            result,
        });
    }

    Ok(results)
}
