//! External 7z binary extraction fallback (e.g. for Rar, Iso formats).

use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::safe_extract::ExtractGuard;
use crate::config::localization::{t, t_fill};
use crate::fs::external_tools::get_external_7z_path;
use crate::fs::progress::ArchiveReport;

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
        .ok_or_else(|| anyhow!(t("archive_7z_listing_unexpected")))?;

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
            return Err(anyhow!(t_fill("archive_entry_under_file", &[&entry.path])));
        }
        guard.preflight(rel, entry.size, entry.is_dir)?;
    }
    Ok(())
}

pub fn extract_via_external_7z(
    archive_path: &Path,
    dest_dir: &Path,
    report: &mut ArchiveReport,
) -> Result<()> {
    let bin_path = get_external_7z_path().ok_or_else(|| anyhow!(t("archive_7z_path_unknown")))?;
    if !bin_path.exists() && cfg!(target_os = "windows") {
        return Err(anyhow!(t("archive_7z_not_downloaded")));
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
        return Err(anyhow!(t("archive_7z_list_failed")));
    }
    let entries = parse_listing(&String::from_utf8_lossy(&list_output.stdout))?;
    if let Err(e) = preflight_listing(&entries, &mut guard) {
        report.failed(archive_path, e.to_string());
        return Err(e);
    }

    report.ensure_running()?;
    // 7z extracts everything at once: the archive is the one entry shown.
    report.entry(&crate::fs::file_name_lossy(archive_path), 0, 1);

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
    // Drained concurrently: an unread stderr pipe fills up (64 KiB) and
    // blocks 7z on its next warning, so the extraction would never end.
    let stderr = drain_stderr(child.stderr.take());

    let status = loop {
        if report.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(crate::fs::error::cancelled());
        }
        match child.try_wait()? {
            Some(status) => break status,
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };

    let stderr = stderr.join().unwrap_or_default();
    match ExtractOutcome::of(status.code()) {
        ExtractOutcome::Failed => {
            let detail = stderr.trim();
            if detail.is_empty() {
                return Err(anyhow!(t("archive_7z_extract_failed")));
            }
            return Err(anyhow!("{}: {detail}", t("archive_7z_extract_failed")));
        }
        ExtractOutcome::Warnings => report.warned(archive_path, "archive_7z_warnings"),
        ExtractOutcome::Clean => {}
    }

    guard.report_skipped(report);
    Ok(())
}

/// How a finished `7z x` run is read from its exit code.
#[derive(Debug, PartialEq)]
enum ExtractOutcome {
    Clean,
    /// Exit code 1: non-fatal warnings (a file was locked, a checksum of a
    /// skipped file...). Everything else was extracted.
    Warnings,
    Failed,
}

impl ExtractOutcome {
    fn of(code: Option<i32>) -> Self {
        match code {
            Some(0) => Self::Clean,
            Some(1) => Self::Warnings,
            _ => Self::Failed,
        }
    }
}

/// Reads the child's stderr to its end on its own thread (lossy UTF-8,
/// capped so a chatty tool cannot grow memory).
fn drain_stderr(pipe: Option<std::process::ChildStderr>) -> std::thread::JoinHandle<String> {
    use std::io::Read;
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else {
            return String::new();
        };
        let mut text = Vec::new();
        let _ = (&mut pipe).take(16 * 1024).read_to_end(&mut text);
        // Keep consuming so the child never blocks on a full pipe.
        let _ = std::io::copy(&mut pipe, &mut std::io::sink());
        String::from_utf8_lossy(&text).into_owned()
    })
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
    fn exit_code_one_is_a_warning_not_a_failure() {
        assert_eq!(ExtractOutcome::of(Some(0)), ExtractOutcome::Clean);
        assert_eq!(ExtractOutcome::of(Some(1)), ExtractOutcome::Warnings);
        assert_eq!(ExtractOutcome::of(Some(2)), ExtractOutcome::Failed);
        assert_eq!(ExtractOutcome::of(None), ExtractOutcome::Failed);
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
