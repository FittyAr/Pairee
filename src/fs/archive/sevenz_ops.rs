//! Native 7z extraction and listing using `sevenz-rust2`.

use anyhow::{Result, anyhow};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use super::safe_extract::ExtractGuard;
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut guard = ExtractGuard::new(dest_dir)?;
    // The extract callback must return a `sevenz_rust2::Error`; keep the
    // real cause here so the user sees why extraction stopped.
    let mut failure: Option<anyhow::Error> = None;

    let result = sevenz_rust2::decompress_file_with_extract_fn(
        archive_path,
        dest_dir,
        |entry, reader, _dest| {
            if ensure_not_cancelled(cancel).is_err() {
                return Ok(false);
            }
            match extract_entry(&mut guard, entry, reader, tx) {
                Ok(()) => Ok(true),
                Err(e) => {
                    let msg = e.to_string();
                    failure = Some(e);
                    Err(sevenz_rust2::Error::Other(msg.into()))
                }
            }
        },
    );

    if let Some(e) = failure {
        let _ = tx.blocking_send(ProgressUpdate {
            skipped: false,
            current_file: String::new(),
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
            error: Some(e.to_string()),
        });
        return Err(e);
    }
    result.map_err(|e| anyhow!("7z extraction failed: {:?}", e))?;
    ensure_not_cancelled(cancel)?;

    guard.report_skipped(tx);
    Ok(())
}

/// Writes one 7z entry through the [`ExtractGuard`] (no overwrite, no
/// writing through links, size/entry limits).
fn extract_entry(
    guard: &mut ExtractGuard,
    entry: &sevenz_rust2::ArchiveEntry,
    reader: &mut dyn std::io::Read,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    guard.count_entry()?;
    let rel = ExtractGuard::sanitize(entry.name())?;
    if rel.as_os_str().is_empty() {
        return Ok(());
    }

    let _ = tx.blocking_send(ProgressUpdate {
        skipped: false,
        current_file: entry.name().to_string(),
        files_copied: 0,
        total_files: 0,
        bytes_copied: 0,
        total_bytes: 0,
        error: None,
    });

    if entry.is_directory() {
        guard.create_dir(&rel)?;
    } else {
        guard.check_declared_size(entry.size())?;
        if let Some(mut out) = guard.create_file(&rel)? {
            guard.copy_limited(reader, &mut out)?;
        }
    }
    Ok(())
}

pub fn list_7z_files(path: &Path) -> Result<Vec<String>> {
    let archive =
        sevenz_rust2::Archive::open(path).map_err(|e| anyhow!("Failed to open 7z: {:?}", e))?;
    let mut list = Vec::new();
    for entry in &archive.files {
        list.push(entry.name().to_string());
    }
    Ok(list)
}
