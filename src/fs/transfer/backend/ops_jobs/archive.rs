//! Archival operation runner (compress, extract) with cooperative cancellation.

use super::super::super::events::TransferEvent;
use super::super::super::job::TransferResults;
use crate::config::localization::t;
use crate::fs::progress::ArchiveReport;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::control::done;
use anyhow::anyhow;
use std::path::PathBuf;

pub async fn run_compress(
    sources: Vec<PathBuf>,
    dest_archive: PathBuf,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    run_archive_blocking(
        control,
        sources.len().max(1),
        move |report| crate::fs::archive::compress_zip(sources, &dest_archive, report),
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
        move |report| crate::fs::archive::extract_archive(&archive, &dest, report),
        "error_extraction_failed",
    )
    .await?;

    if results.completed_files.is_empty() && results.failed_files.is_empty() {
        results.completed_files.push(done(
            &archive_for_complete,
            destination_dir,
            0,
            std::time::Instant::now(),
        ));
    }
    Ok(results)
}

/// Runs a blocking archive operation that reports to the job through an
/// [`ArchiveReport`].
pub(in crate::fs::transfer::backend) async fn run_archive_blocking<F>(
    control: JobControl,
    default_total: usize,
    work: F,
    err_key: &str,
) -> Result<TransferResults, anyhow::Error>
where
    F: FnOnce(&mut ArchiveReport) -> anyhow::Result<()> + Send + 'static,
{
    let ctl = control.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let mut report = ArchiveReport::new(ctl, default_total);
        let outcome = work(&mut report);
        (outcome, report.into_results())
    });
    match handle.await {
        Ok((Ok(()), results)) => {
            if control.is_cancelled() {
                return Err(crate::fs::error::cancelled());
            }
            control.job_completed(results)
        }
        Ok((Err(e), _)) => {
            if control.is_cancelled() {
                return Err(crate::fs::error::cancelled());
            }
            let err_msg = t(err_key).replacen("{}", &e.to_string(), 1);
            control.emit(TransferEvent::JobFailed {
                job_id: control.job_id,
                error: err_msg.clone(),
            });
            Err(anyhow!(err_msg))
        }
        Err(e) => Err(anyhow!("archive task join error: {e}")),
    }
}
