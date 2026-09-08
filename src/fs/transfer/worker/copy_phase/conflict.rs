//! Destination collision resolution and conflict handling for copy_phase.

use super::super::super::conflict::resolve_filename_conflict;
use super::super::super::events::TransferEvent;
use super::super::super::job::{SkippedFile, TransferResults};
use super::super::super::options::TransferOptions;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

pub enum ConflictAction {
    Skip,
    Proceed,
}

pub async fn resolve_existing_destination(
    src: &Path,
    dst: &mut PathBuf,
    options: &TransferOptions,
    job_id: Uuid,
    is_cancelled: &AtomicBool,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    active_conflict: &Arc<
        std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>,
    >,
    auto_resolution: &mut Option<crate::fs::transfer::conflict::ConflictResolution>,
    results: &mut TransferResults,
) -> Result<ConflictAction, anyhow::Error> {
    let mut resolution = options.conflict_resolution.clone();
    if resolution == "ask" {
        let chosen = if let Some(auto_res) = *auto_resolution {
            auto_res
        } else {
            let _ = event_tx.send(TransferEvent::ConflictDetected {
                job_id,
                file: dst.clone(),
                conflict: crate::fs::transfer::conflict::ConflictInfo {
                    src_path: src.to_path_buf(),
                    dst_path: dst.clone(),
                    src_size: src.metadata().map(|m| m.len()).unwrap_or(0),
                    dst_size: dst.metadata().map(|m| m.len()).unwrap_or(0),
                    src_modified: src.metadata().and_then(|m| m.modified()).ok(),
                    dst_modified: dst.metadata().and_then(|m| m.modified()).ok(),
                },
            });

            {
                let mut guard = active_conflict.lock().unwrap();
                *guard = None;
            }

            while active_conflict.lock().unwrap().is_none() {
                if is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            let ch = (*active_conflict.lock().unwrap())
                .unwrap_or(crate::fs::transfer::conflict::ConflictResolution::Skip);
            match ch {
                crate::fs::transfer::conflict::ConflictResolution::OverwriteAll
                | crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll
                | crate::fs::transfer::conflict::ConflictResolution::SkipAll
                | crate::fs::transfer::conflict::ConflictResolution::RenameAll => {
                    *auto_resolution = Some(ch);
                }
                _ => {}
            }
            ch
        };

        resolution = match chosen {
            crate::fs::transfer::conflict::ConflictResolution::Overwrite
            | crate::fs::transfer::conflict::ConflictResolution::OverwriteAll => {
                "overwrite".to_string()
            }
            crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder
            | crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll => {
                "overwrite_older".to_string()
            }
            crate::fs::transfer::conflict::ConflictResolution::Rename
            | crate::fs::transfer::conflict::ConflictResolution::RenameAll
            | crate::fs::transfer::conflict::ConflictResolution::KeepBoth => "rename".to_string(),
            crate::fs::transfer::conflict::ConflictResolution::Cancel => {
                is_cancelled.store(true, Ordering::SeqCst);
                return Err(anyhow!("Job cancelled"));
            }
            _ => "skip".to_string(),
        };
    }

    match resolution.as_str() {
        "skip" => {
            results.skipped_files.push(SkippedFile {
                src: src.to_path_buf(),
                reason: "File already exists (skipped)".to_string(),
            });
            let _ = event_tx.send(TransferEvent::FileSkipped {
                job_id,
                file: src.to_path_buf(),
                reason: "File already exists".to_string(),
            });
            Ok(ConflictAction::Skip)
        }
        "rename" | "keep_both" => {
            *dst = resolve_filename_conflict(dst);
            Ok(ConflictAction::Proceed)
        }
        "overwrite_older" => {
            let src_time = src.metadata().and_then(|m| m.modified()).ok();
            let dst_time = dst.metadata().and_then(|m| m.modified()).ok();
            if let (Some(s_time), Some(d_time)) = (src_time, dst_time)
                && s_time <= d_time
            {
                results.skipped_files.push(SkippedFile {
                    src: src.to_path_buf(),
                    reason: "Destination is newer or equal (skipped)".to_string(),
                });
                let _ = event_tx.send(TransferEvent::FileSkipped {
                    job_id,
                    file: src.to_path_buf(),
                    reason: "Destination is newer or equal".to_string(),
                });
                return Ok(ConflictAction::Skip);
            }
            Ok(ConflictAction::Proceed)
        }
        _ => Ok(ConflictAction::Proceed), // Overwrite
    }
}
