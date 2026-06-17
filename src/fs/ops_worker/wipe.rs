use super::ProgressUpdate;
use crate::config::localization::t;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Spawns a background task that securely wipes each file in `targets`.
/// Uses the same progress channel pattern as `spawn_copy_task`.
pub fn spawn_wipe_task(targets: Vec<PathBuf>) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(64);
    let total = targets.len();

    tokio::task::spawn_blocking(move || {
        for (idx, path) in targets.iter().enumerate() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());

            let _ = tx.blocking_send(ProgressUpdate {
                current_file: name.clone(),
                files_copied: idx,
                total_files: total,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });

            if let Err(e) = crate::fs::wipe::wipe_file(path) {
                let err_msg = t("error_wipe_failed_for")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                let _ = tx.blocking_send(ProgressUpdate {
                    current_file: "Completed".to_string(),
                    files_copied: idx,
                    total_files: total,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: Some(err_msg),
                });
                return;
            }
        }

        let _ = tx.blocking_send(ProgressUpdate {
            current_file: "Completed".to_string(),
            files_copied: total,
            total_files: total,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });
    });

    rx
}
