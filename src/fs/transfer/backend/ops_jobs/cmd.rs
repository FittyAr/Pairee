//! Shell command application runner.

use super::super::super::events::TransferEvent;
use super::super::super::job::{FileTransferResult, TransferResults};
use super::super::BackendControl;
use super::common::{
    complete_ok, emit_file_completed, emit_file_started, emit_scan_complete, fail_file,
};
use anyhow::anyhow;
use std::path::PathBuf;
use std::time::Instant;

pub async fn run_apply_command(
    sources: Vec<PathBuf>,
    cmd_template: String,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let total = sources.len();
    emit_scan_complete(&control, total, 0);

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
        emit_file_started(&control, path, idx);

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
                emit_file_completed(&control, result);
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

pub(crate) async fn run_shell_command(cmd: &str) -> anyhow::Result<String> {
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
