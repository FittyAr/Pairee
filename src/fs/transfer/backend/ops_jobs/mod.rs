//! Wipe / Compress / Extract / ApplyCommand backends for the Transfer Engine.
//!
//! Pattern: **Strategy** (per-op runner) with cooperative cancel via
//! [crate::fs::progress::ensure_not_cancelled] inside archive loops.

mod archive;
mod cmd;
mod common;
mod wipe;

use super::super::job::{TransferOperation, TransferResults};
use super::BackendControl;
use anyhow::anyhow;
use std::path::PathBuf;

pub async fn run_ops_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    shell_template: Option<String>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    match operation {
        TransferOperation::Wipe => wipe::run_wipe(sources, control).await,
        TransferOperation::Compress => archive::run_compress(sources, destination, control).await,
        TransferOperation::Extract => archive::run_extract(sources, destination, control).await,
        TransferOperation::ApplyCommand => {
            let template = shell_template
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow!("ApplyCommand requires a shell template"))?;
            cmd::run_apply_command(sources, template, control).await
        }
        other => Err(anyhow!("ops backend does not handle {}", other.label())),
    }
}

#[cfg(test)]
mod tests {
    use super::cmd::{run_shell_command, split_command_output};

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

    #[tokio::test]
    async fn test_run_shell_command_captures_echo() {
        let text = run_shell_command("echo hello")
            .await
            .expect("echo should succeed");
        assert!(
            text.to_lowercase().contains("hello"),
            "expected echo output, got {text:?}"
        );
    }
}
