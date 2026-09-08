//! Single file transfer executor with retry backoff and symlink support.

use super::super::super::events::TransferEvent;
use super::super::super::options::TransferOptions;
use super::super::super::pipeline::copy_file_pipelined;
use anyhow::anyhow;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct TransferOutcome {
    pub success: bool,
    pub last_error: String,
    pub retries: u32,
    pub src_hash: Option<String>,
    pub dst_hash: Option<String>,
}

pub async fn transfer_one_file(
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<AtomicU64>,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
) -> Result<TransferOutcome, anyhow::Error> {
    let mut retries = 0u32;
    let mut copy_success = false;
    let mut last_error = String::new();
    let mut src_hash = None;
    let mut dst_hash = None;

    let is_symlink = src.is_symlink();
    let recreate_link = is_symlink && !options.follow_symlinks;

    if recreate_link {
        match recreate_symlink(src, dst) {
            Ok(()) => copy_success = true,
            Err(e) => last_error = format!("Error creating symlink: {}", e),
        }
    } else {
        while retries <= options.max_retries {
            if is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }

            match copy_file_pipelined(
                src,
                dst,
                options,
                event_tx,
                job_id,
                Arc::clone(&is_paused),
                Arc::clone(&is_cancelled),
                Arc::clone(&bytes_transferred_acc),
            )
            .await
            {
                Ok((s_hash, d_hash)) => {
                    src_hash = s_hash;
                    dst_hash = d_hash;
                    copy_success = true;
                    break;
                }
                Err(e) => {
                    retries += 1;
                    last_error = e.to_string();
                    if retries <= options.max_retries {
                        let shift = retries.min(20);
                        let backoff_ms = 100u64.saturating_mul(1u64 << shift);
                        tokio::time::sleep(Duration::from_millis(backoff_ms.min(30_000))).await;
                    }
                }
            }
        }
    }

    Ok(TransferOutcome {
        success: copy_success,
        last_error,
        retries,
        src_hash,
        dst_hash,
    })
}

fn recreate_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    let target = std::fs::read_link(src)?;
    let resolved_target = if target.is_relative() {
        src.parent()
            .map(|p| p.join(&target))
            .unwrap_or_else(|| target.clone())
    } else {
        target.clone()
    };
    #[cfg(target_os = "windows")]
    {
        let is_dir = resolved_target.is_dir();
        if dst.exists() {
            let _ = std::fs::remove_file(dst);
            let _ = std::fs::remove_dir_all(dst);
        }
        if is_dir {
            std::os::windows::fs::symlink_dir(&resolved_target, dst)?;
        } else {
            std::os::windows::fs::symlink_file(&resolved_target, dst)?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if dst.exists() {
            let _ = std::fs::remove_file(dst);
        }
        std::os::unix::fs::symlink(&resolved_target, dst)?;
    }
    Ok(())
}
