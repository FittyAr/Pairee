//! Archive operation runner (compress, and the archive jobs of
//! [`super::super::archive_vfs`]) with cooperative cancellation.

use super::super::super::events::TransferEvent;
use super::super::super::job::TransferResults;
use crate::config::localization::t;
use crate::fs::progress::ArchiveReport;
use crate::fs::transfer::control::JobControl;
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
