//! Tar and Tar.gz extraction operations.

use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tar::Archive;
use tokio::sync::mpsc;

use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_tar_gz(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let tar_gz = fs::File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    fs::create_dir_all(dest_dir)?;

    for (i, entry) in archive.entries()?.enumerate() {
        ensure_not_cancelled(cancel)?;
        let mut file = entry?;
        let path = file.path()?;

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let _ = tx.blocking_send(ProgressUpdate {
            current_file: file_name,
            files_copied: i,
            total_files: 0, // Unknown without pre-scanning
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        file.unpack_in(dest_dir)?;
    }

    Ok(())
}

pub fn list_tar_gz_files(path: &Path) -> Result<Vec<String>> {
    let tar_gz = fs::File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    let mut list = Vec::new();
    for entry in archive.entries()?.flatten() {
        if let Ok(path) = entry.path() {
            list.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(list)
}
