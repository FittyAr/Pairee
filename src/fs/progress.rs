//! Shared progress + cooperative cancel for long-running FS jobs.
//!
//! Used by archive compress/extract (and similar) so Transfer Engine backends
//! can observe progress and stop mid-operation without a second UI stack.

use std::sync::atomic::{AtomicBool, Ordering};

/// Progress tick shared by archive helpers and Transfer adapters.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub current_file: String,
    pub files_copied: usize,
    pub total_files: usize,
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

/// Returns `Err` when the job was cancelled (cooperative cancel point).
pub fn ensure_not_cancelled(cancel: &AtomicBool) -> anyhow::Result<()> {
    if cancel.load(Ordering::Relaxed) {
        anyhow::bail!("Job cancelled");
    }
    Ok(())
}
