//! Shell command application runner.

use super::super::super::events::TransferEvent;
use super::super::super::job::TransferResults;
use crate::fs::transfer::control::JobControl;
use std::path::PathBuf;

/// Runs the command template on each source in turn (blocking), forwarding
/// its output; the first failure fails the job.
pub fn run_apply_command(
    sources: Vec<PathBuf>,
    cmd_template: String,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    control.run_each(&sources, |path| {
        let cmd = cmd_template.replace("%f", &crate::shell::quote_path(path));
        emit_command_output(&control, &format!("$ {cmd}"));
        match run_shell_command_blocking(&cmd) {
            Ok(text) => {
                emit_command_output(&control, &text);
                Ok(PathBuf::new())
            }
            Err(e) => {
                emit_command_output(&control, &e.to_string());
                Err(format!("Command failed for {:?}: {}", path, e))
            }
        }
    })
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

fn emit_command_output(control: &JobControl, text: &str) {
    for line in split_command_output(text) {
        let _ = control.event_tx.send(TransferEvent::CommandOutput {
            job_id: control.job_id,
            line,
        });
    }
}

/// Runs `cmd` in a shell on a pseudo-terminal; `Err` with its output when
/// it fails.
pub(crate) fn run_shell_command_blocking(cmd: &str) -> anyhow::Result<String> {
    let result = crate::terminal::pty_cmd::run_shell_on_pty(cmd, None)?;
    if !result.success {
        anyhow::bail!("{}", result.output.trim());
    }
    Ok(result.output)
}
