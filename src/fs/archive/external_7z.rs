//! External 7z binary extraction fallback (e.g. for Rar, Iso formats).

use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use super::safe_extract::ExtractGuard;
use crate::fs::external_tools::get_external_7z_path;
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

/// One entry of `7z l -slt` output.
#[derive(Debug, PartialEq)]
struct ListedEntry {
    path: String,
    size: u64,
    is_dir: bool,
}

/// Parses the technical listing (`7z l -slt`). Only blocks after the
/// `----------` separator describe entries; the block before it describes
/// the archive itself.
fn parse_listing(stdout: &str) -> Result<Vec<ListedEntry>> {
    let text = stdout.replace("\r\n", "\n");
    let (_, body) = text
        .split_once("\n----------\n")
        .ok_or_else(|| anyhow!("Unexpected 7z listing format"))?;

    let mut entries = Vec::new();
    for block in body.split("\n\n") {
        let mut path = None;
        let mut size = 0u64;
        let mut is_dir = false;
        for line in block.lines() {
            if let Some(v) = line.strip_prefix("Path = ") {
                path = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("Size = ") {
                size = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("Folder = ") {
                is_dir = v.trim() == "+";
            } else if let Some(v) = line.strip_prefix("Attributes = ") {
                is_dir |= v.trim_start().starts_with('D');
            }
        }
        if let Some(path) = path {
            entries.push(ListedEntry { path, size, is_dir });
        }
    }
    Ok(entries)
}

/// Validates every listed entry before 7z is allowed to write anything:
/// path traversal, existing links in the destination, entries nested under
/// a non-directory entry (e.g. an archived symlink), and size/count limits.
fn preflight_listing(entries: &[ListedEntry], guard: &mut ExtractGuard) -> Result<()> {
    let mut leaves: HashSet<PathBuf> = HashSet::new();
    let mut rels = Vec::with_capacity(entries.len());
    for entry in entries {
        let rel = ExtractGuard::sanitize(&entry.path)?;
        if !entry.is_dir {
            leaves.insert(rel.clone());
        }
        rels.push(rel);
    }
    for (entry, rel) in entries.iter().zip(&rels) {
        if rel.ancestors().skip(1).any(|a| leaves.contains(a)) {
            return Err(anyhow!(
                "Refusing to extract archive: entry {} is nested under a non-directory entry",
                entry.path
            ));
        }
        guard.preflight(rel, entry.size, entry.is_dir)?;
    }
    Ok(())
}

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

    let mut guard = ExtractGuard::new(dest_dir)?;

    // Fail closed: without a successful listing nothing has been validated,
    // so the archive is never handed to 7z for extraction.
    let list_output = std::process::Command::new(&bin_path)
        .arg("l")
        .arg("-slt")
        .arg(archive_path)
        .output()?;
    if !list_output.status.success() {
        return Err(anyhow!(
            "Refusing to extract archive: 7z could not list its contents"
        ));
    }
    let entries = parse_listing(&String::from_utf8_lossy(&list_output.stdout))?;
    if let Err(e) = preflight_listing(&entries, &mut guard) {
        let _ = tx.blocking_send(ProgressUpdate {
            skipped: false,
            current_file: archive_path.to_string_lossy().into_owned(),
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
            error: Some(e.to_string()),
        });
        return Err(e);
    }

    ensure_not_cancelled(cancel)?;
    let _ = tx.blocking_send(ProgressUpdate {
        skipped: false,
        current_file: "Extracting using external 7z...".to_string(),
        files_copied: 0,
        total_files: 0,
        bytes_copied: 0,
        total_bytes: 0,
        error: None,
    });

    // `-aos`: skip files that already exist instead of overwriting them.
    let mut child = std::process::Command::new(&bin_path)
        .arg("x")
        .arg("-y")
        .arg("-aos")
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

    guard.report_skipped(tx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "7-Zip 26.01\r\n\r\nListing archive: C:\\a.rar\r\n\r\n--\r\nPath = C:\\a.rar\r\nType = Rar\r\n\r\n----------\r\nPath = docs\r\nFolder = +\r\nSize = 0\r\n\r\nPath = docs\\readme.txt\r\nFolder = -\r\nSize = 12\r\n\r\n";

    #[test]
    fn parse_listing_skips_archive_header() {
        let entries = parse_listing(LISTING).unwrap();
        assert_eq!(
            entries,
            vec![
                ListedEntry {
                    path: "docs".into(),
                    size: 0,
                    is_dir: true
                },
                ListedEntry {
                    path: "docs\\readme.txt".into(),
                    size: 12,
                    is_dir: false
                },
            ]
        );
    }

    #[test]
    fn parse_listing_fails_closed_on_garbage() {
        assert!(parse_listing("error: cannot open").is_err());
    }

    #[test]
    fn preflight_rejects_traversal_nesting_and_bombs() {
        let dir = tempfile::tempdir().unwrap();
        let entry = |path: &str, size, is_dir| ListedEntry {
            path: path.into(),
            size,
            is_dir,
        };

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        assert!(preflight_listing(&[entry("../x", 1, false)], &mut guard).is_err());

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        let nested = [entry("link", 0, false), entry("link/pwn", 1, false)];
        assert!(preflight_listing(&nested, &mut guard).is_err());

        let mut guard = ExtractGuard::with_limits(dir.path(), 100, 10).unwrap();
        assert!(preflight_listing(&[entry("big", 101, false)], &mut guard).is_err());

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        let ok = [entry("docs", 0, true), entry("docs/a.txt", 5, false)];
        assert!(preflight_listing(&ok, &mut guard).is_ok());
    }
}
