use super::ProgressUpdate;
use crate::config::localization::t;
use crate::fs::archive::extract_archive;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub fn spawn_extract_task(
    archive_path: PathBuf,
    destination_dir: PathBuf,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || {
        if let Err(e) = extract_archive(&archive_path, &destination_dir, &tx) {
            let err_msg = t("error_extraction_failed").replacen("{}", &e.to_string(), 1);
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: archive_path.to_string_lossy().into_owned(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(err_msg),
            });
        } else {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied: 1,
                total_files: 1,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });
        }
    });
    rx
}
