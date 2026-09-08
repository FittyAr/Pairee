//! External 7z binary extraction fallback (e.g. for Rar, Iso formats).

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Component, Path};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::fs::external_tools::get_external_7z_path;
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_via_external_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let bin_path = get_external_7z_path().ok_or_else(|| anyhow!("Could not determine 7z path"))?;
    if !bin_path.exists() && cfg!(target_os = "windows") {
        return Err(anyhow!(
            "7z tool is not downloaded yet. Please wait for the background download to finish."
        ));
    }

    fs::create_dir_all(dest_dir)?;

    let list_output = std::process::Command::new(&bin_path)
        .arg("l")
        .arg("-slt")
        .arg(archive_path)
        .output()?;
    if list_output.status.success() {
        let listing = String::from_utf8_lossy(&list_output.stdout);
        for block in listing.split("\n\n") {
            for line in block.lines() {
                if let Some(path) = line.strip_prefix("Path = ") {
                    let candidate = std::path::Path::new(path.trim());
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
                        let _ = tx.blocking_send(ProgressUpdate {
                            current_file: path.trim().to_string(),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: Some(format!(
                                "Refusing to extract archive: entry {} would escape the destination",
                                path.trim()
                            )),
                        });
                        return Err(anyhow!(
                            "Refusing to extract archive: entry {} contains a path-traversal component",
                            path.trim()
                        ));
                    }
                }
            }
        }
    }

    ensure_not_cancelled(cancel)?;
    let _ = tx.blocking_send(ProgressUpdate {
        current_file: "Extracting using external 7z...".to_string(),
        files_copied: 0,
        total_files: 0,
        bytes_copied: 0,
        total_bytes: 0,
        error: None,
    });

    let mut child = std::process::Command::new(&bin_path)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", dest_dir.to_string_lossy()))
        .arg(archive_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let status = loop {
        if ensure_not_cancelled(cancel).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!("Job cancelled"));
        }
        match child.try_wait()? {
            Some(status) => break status,
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };

    if !status.success() {
        return Err(anyhow!("External 7z extraction failed"));
    }

    Ok(())
}
