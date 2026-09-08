//! Wipe / Compress / Extract / ApplyCommand backends for the Transfer Engine.
//!
//! Pattern: **Strategy** (per-op runner) with cooperative cancel via
//! [`crate::fs::progress::ensure_not_cancelled`] inside archive loops.

use super::super::events::TransferEvent;
use super::super::job::{FailedFile, FileTransferResult, TransferOperation, TransferResults};
use super::BackendControl;
use crate::config::localization::t;
use crate::fs::progress::ProgressUpdate;
use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use tokio::sync::mpsc;

pub async fn run_ops_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    shell_template: Option<String>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    match operation {
        TransferOperation::Wipe => run_wipe(sources, control).await,
        TransferOperation::Compress => run_compress(sources, destination, control).await,
        TransferOperation::Extract => run_extract(sources, destination, control).await,
        TransferOperation::ApplyCommand => {
            let template = shell_template
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow!("ApplyCommand requires a shell template"))?;
            run_apply_command(sources, template, control).await
        }
        other => Err(anyhow!("ops backend does not handle {}", other.label())),
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
                return fail_file(&control, &mut results, path.clone(), err_msg);
            }
        }
    }

    complete_ok(&control, results)
}

async fn run_compress(
    sources: Vec<PathBuf>,
    dest_archive: PathBuf,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    run_archive_blocking(
        control,
        sources.len().max(1),
        move |tx, cancel| crate::fs::archive::compress_zip(sources, &dest_archive, tx, cancel),
        "error_compression_failed",
    )
    .await
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
        results.completed_files.push(FileTransferResult {
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

async fn run_apply_command(
    sources: Vec<PathBuf>,
    cmd_template: String,
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

        let quoted = crate::app::actions::fs_ops::helper::shell_quote(path);
        let cmd = cmd_template.replace("%f", &quoted);
        emit_command_output(&control, &format!("$ {cmd}"));

        match run_shell_command(&cmd).await {
            Ok(text) => {
                emit_command_output(&control, &text);
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
                emit_command_output(&control, &e.to_string());
                let err_msg = format!("Command failed for {:?}: {}", path, e);
                return fail_file(&control, &mut results, path.clone(), err_msg);
            }
        }
    }

    complete_ok(&control, results)
}

/// Split captured command output into terminal lines (strip CR, drop a trailing empty line).
pub(crate) fn split_command_output(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let text = text.trim_end_matches(['\n', '\r']);
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect()
}

fn emit_command_output(control: &BackendControl, text: &str) {
    for line in split_command_output(text) {
        let _ = control.event_tx.send(TransferEvent::CommandOutput {
            job_id: control.job_id,
            line,
        });
    }
}

#[cfg(test)]
fn combine_stdio(stdout: &[u8], stderr: &[u8]) -> String {
    let out = String::from_utf8_lossy(stdout);
    let err = String::from_utf8_lossy(stderr);
    match (out.is_empty(), err.is_empty()) {
        (true, true) => String::new(),
        (false, true) => out.into_owned(),
        (true, false) => err.into_owned(),
        (false, false) => format!("{out}{err}"),
    }
}

async fn run_shell_command(cmd: &str) -> anyhow::Result<String> {
    let cmd = cmd.to_string();
    let result =
        tokio::task::spawn_blocking(move || crate::terminal::pty_cmd::run_shell_on_pty(&cmd, None))
            .await
            .map_err(|e| anyhow!("pty task join error: {e}"))??;
    if !result.success {
        anyhow::bail!("{}", result.output.trim());
    }
    Ok(result.output)
}

async fn run_archive_blocking<F>(
    control: BackendControl,
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
            if control.cancelled() {
                return Err(anyhow!("Job cancelled"));
            }
            complete_ok(&control, results)
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.to_lowercase().contains("cancel") || control.cancelled() {
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
    control: &BackendControl,
    default_total: usize,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let mut announced_scan = false;

    while let Some(update) = rx.recv().await {
        if control.cancelled() {
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
            let _ = control.event_tx.send(TransferEvent::ScanComplete {
                job_id: control.job_id,
                total_files: total,
                total_bytes: update.total_bytes,
            });
        }

        if let Some(err) = update.error {
            let path = PathBuf::from(&update.current_file);
            let failed = FailedFile {
                src: path,
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

        let path = PathBuf::from(&update.current_file);
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: path.clone(),
            index: update.files_copied,
        });
        if update.total_bytes > 0 {
            let _ = control.event_tx.send(TransferEvent::FileProgress {
                job_id: control.job_id,
                bytes_copied: update.bytes_copied,
                bytes_total: update.total_bytes,
            });
        }

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

fn fail_file(
    control: &BackendControl,
    results: &mut TransferResults,
    path: PathBuf,
    err_msg: String,
) -> Result<TransferResults, anyhow::Error> {
    let failed = FailedFile {
        src: path,
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
    Err(anyhow!(err_msg))
}

fn complete_ok(
    control: &BackendControl,
    results: TransferResults,
) -> Result<TransferResults, anyhow::Error> {
    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{combine_stdio, split_command_output};

    #[test]
    fn split_command_output_strips_cr_and_keeps_sgr() {
        let lines = split_command_output("\x1b[31mred\x1b[0m\r\nnext\n");
        assert_eq!(
            lines,
            vec!["\x1b[31mred\x1b[0m".to_string(), "next".to_string()]
        );
    }

    #[test]
    fn split_command_output_empty_is_empty() {
        assert!(split_command_output("").is_empty());
        assert!(split_command_output("\n").is_empty());
    }

    #[test]
    fn combine_stdio_joins_stdout_then_stderr() {
        assert_eq!(combine_stdio(b"out\n", b"err\n"), "out\nerr\n");
        assert_eq!(combine_stdio(b"", b"err"), "err");
        assert!(combine_stdio(b"", b"").is_empty());
    }

    #[tokio::test]
    async fn run_shell_command_captures_echo() {
        let text = super::run_shell_command("echo hello")
            .await
            .expect("echo should succeed");
        assert!(
            text.to_lowercase().contains("hello"),
            "expected echo output, got {text:?}"
        );
    }
}
