//! Native 7z extraction and listing using `sevenz-rust2`.

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Component, Path};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    fs::create_dir_all(dest_dir)?;
    // Canonicalise the destination so we can verify that no extracted
    // entry escapes it via `..` components or absolute paths.
    let canonical_dest = std::fs::canonicalize(dest_dir).unwrap_or_else(|_| dest_dir.to_path_buf());

    sevenz_rust2::decompress_file_with_extract_fn(archive_path, dest_dir, |entry, reader, dest| {
        if ensure_not_cancelled(cancel).is_err() {
            return Ok(false);
        }
        // `sevenz-rust2` still joins `dest` with `entry.name()` before calling
        // the extract function. A malicious 7z archive can write outside the
        // destination via `..` or absolute paths. Refuse those entries.
        let entry_name = entry.name();
        let candidate = std::path::Path::new(entry_name);
        let mut has_traversal = false;
        for component in candidate.components() {
            match component {
                Component::ParentDir => {
                    has_traversal = true;
                    break;
                }
                Component::Prefix(_) | Component::RootDir => {
                    has_traversal = true;
                    break;
                }
                _ => {}
            }
        }
        if has_traversal {
            let file_name = entry.name().to_string();
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: file_name,
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!(
                    "Refusing to extract entry with unsafe path: {}",
                    entry.name()
                )),
            });
            return Ok(false);
        }

        let dest_path = dest.to_path_buf();
        let check_target = dest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dest_path.clone());
        if let Ok(canon) = std::fs::canonicalize(&check_target)
            && !canon.starts_with(&canonical_dest)
        {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: entry.name().to_string(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!(
                    "Refusing to extract entry outside destination: {}",
                    entry.name()
                )),
            });
            return Ok(false);
        }

        let file_name = entry.name().to_string();
        let _ = tx.blocking_send(ProgressUpdate {
            current_file: file_name,
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        sevenz_rust2::default_entry_extract_fn(entry, reader, &dest_path)
    })
    .map_err(|e| anyhow!("7z extraction failed: {:?}", e))?;

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
