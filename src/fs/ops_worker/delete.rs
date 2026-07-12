use crate::fs::ops_worker::ProgressUpdate;
use crate::fs::ssh::SharedSshClient;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Spawns a background task to delete remote files/directories over SSH SFTP.
pub fn spawn_ssh_delete_task(
    client: SharedSshClient,
    targets: Vec<PathBuf>,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(64);
    let total = targets.len();

    tokio::spawn(async move {
        for (idx, path) in targets.iter().enumerate() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());

            let _ = tx.send(ProgressUpdate {
                current_file: name.clone(),
                files_copied: idx,
                total_files: total,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            }).await;

            if let Err(e) = client.delete_recursive(path) {
                let _ = tx.send(ProgressUpdate {
                    current_file: "Completed".to_string(),
                    files_copied: idx,
                    total_files: total,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: Some(e.to_string()),
                }).await;
                return;
            }
        }

        let _ = tx.send(ProgressUpdate {
            current_file: "Completed".to_string(),
            files_copied: total,
            total_files: total,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        }).await;
    });

    rx
}
