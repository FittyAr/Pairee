//! UI-facing facade to enqueue transfer jobs on the engine.

use super::engine::TransferEngine;
use super::job::{SshEndpoints, TransferJob, TransferOperation};
use super::options::TransferOptions;
use crate::app::state::{AppState, TransferViewMode};
use crate::fs::ssh::SharedSshClient;
use std::path::PathBuf;

/// Ensure the transfer UI + engine exist on `state`.
pub fn ensure_transfer_ui(state: &mut AppState) {
    if state.transfer.is_none() {
        let (engine, rx) = TransferEngine::new();
        state.transfer = Some(crate::app::state::transfer_state::TransferUIState::new(
            engine, rx,
        ));
    }
}

/// Submit a job (local or SSH) and show the minimized transfer bar.
pub fn submit_job(state: &mut AppState, job: TransferJob) {
    for src in &job.sources {
        super::history::add_source_path(src);
    }
    if !job.destination.as_os_str().is_empty() {
        super::history::add_dest_path(&job.destination);
    }

    ensure_transfer_ui(state);
    if let Some(ref mut ts) = state.transfer {
        ts.engine.submit_job(job);
        ts.view_mode = TransferViewMode::Minimized;
    }
    state.active_popup = None;
}

/// Convenience: enqueue a job with optional SSH endpoints.
pub fn submit_simple(
    state: &mut AppState,
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    options: TransferOptions,
    src_ssh: Option<SharedSshClient>,
    dst_ssh: Option<SharedSshClient>,
) {
    let mut job = TransferJob::new(operation, sources, destination, options);
    if src_ssh.is_some() || dst_ssh.is_some() {
        job = job.with_ssh(SshEndpoints {
            src: src_ssh,
            dst: dst_ssh,
        });
    }
    submit_job(state, job);
}

/// Enqueue ApplyCommand (`%f` = each source path) on the Transfer Engine UI.
pub fn submit_apply_command(state: &mut AppState, template: String, targets: Vec<PathBuf>) {
    let job = TransferJob::new(
        TransferOperation::ApplyCommand,
        targets,
        PathBuf::new(),
        TransferOptions::default(),
    )
    .with_shell_template(template);
    submit_job(state, job);
}
