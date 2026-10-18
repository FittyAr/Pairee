//! Formats read through an external 7-Zip (Strategy for [`ArchiveReader`]):
//! RAR, ISO images, CAB, WIM and the other formats without a native
//! reader, and 7z archives the native reader cannot decode (see
//! [`super::fallback`]).
//!
//! Entries are listed with `7z l -slt`. Reading them unpacks only the
//! wanted ones into a private temporary folder (validated first: no path
//! traversal, no entry below a file entry, size and entry limits), whose
//! files are then streamed to the visitor in archive order; the folder
//! goes away afterwards. The visitor writes through the same
//! [`super::safe_extract::ExtractGuard`] as every native format.

use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit, Visitor};
use super::safe_extract::ExtractGuard;
use crate::config::localization::{t, t_fill};
use crate::fs::external_tools::{SevenZipTool, find_7z};

pub struct External7zReader;

/// One entry of `7z l -slt` output.
#[derive(Debug, PartialEq)]
struct ListedEntry {
    path: String,
    size: u64,
    is_dir: bool,
    modified: Option<std::time::SystemTime>,
}

impl ListedEntry {
    fn meta(&self) -> EntryMeta {
        EntryMeta {
            name: self.path.clone(),
            kind: if self.is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            },
            size: self.size,
            modified: self.modified,
            mode: None,
        }
    }
}

/// `7z` reads its local times as `2026-10-18 08:40:49[.fraction]`.
fn listed_time(value: &str) -> Option<std::time::SystemTime> {
    let seconds = value.get(..19)?;
    let naive = chrono::NaiveDateTime::parse_from_str(seconds, "%Y-%m-%d %H:%M:%S").ok()?;
    let local = naive.and_local_timezone(chrono::Local).earliest()?;
    Some(local.into())
}

/// Parses the technical listing (`7z l -slt`). Only blocks after the
/// `----------` separator describe entries; the block before it describes
/// the archive itself. Alternate data streams (`file:stream`) are left
/// out: they are not files of their own.
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
        let mut modified = None;
        let mut stream = false;
        for line in block.lines() {
            let Some((key, value)) = line.split_once(" = ").or_else(|| {
                // `Size = ` with nothing after it loses the trailing space.
                line.strip_suffix(" =").map(|key| (key, ""))
            }) else {
                continue;
            };
            let value = value.trim();
            match key {
                "Path" => path = Some(value.to_string()),
                "Size" => size = value.parse().unwrap_or(0),
                "Folder" => is_dir |= value == "+",
                "Attributes" => is_dir |= value.starts_with('D'),
                "Modified" => modified = listed_time(value),
                "Alternate Stream" => stream = value == "+",
                _ => {}
            }
        }
        if let Some(path) = path.filter(|p| !p.is_empty() && !stream) {
            entries.push(ListedEntry {
                path,
                size,
                is_dir,
                modified,
            });
        }
    }
    Ok(entries)
}

/// Validates every listed entry before 7z is allowed to write anything:
/// path traversal, existing links in the destination, entries nested under
/// a non-directory entry (e.g. an archived symlink), and size/count limits.
fn preflight_listing(entries: &[&ListedEntry], guard: &mut ExtractGuard) -> Result<()> {
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

/// The 7-Zip to run, or why there is none.
fn tool() -> Result<SevenZipTool> {
    find_7z().ok_or_else(|| anyhow!(t("archive_7z_missing")))
}

/// A `7z` command that never waits on the console: no password prompt
/// (stdin is closed), UTF-8 names in and out.
fn command(tool: &SevenZipTool, op: &str) -> Command {
    let mut cmd = Command::new(&tool.path);
    cmd.arg(op).arg("-y").arg("-sccUTF-8").stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up for every listing.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Why `tool` could not read `archive`: its own message, and a hint when
/// the bundled reduced 7-Zip (no RAR, ISO...) is all there is.
fn tool_error(tool: &SevenZipTool, key: &str, stderr: &[u8]) -> anyhow::Error {
    let detail = String::from_utf8_lossy(stderr);
    let detail = detail
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut message = t(key);
    if !detail.is_empty() {
        message = format!("{message}: {detail}");
    }
    if !tool.full {
        message = format!("{message}\n{}", t("archive_7z_full_needed"));
    }
    anyhow!(message)
}

fn list(tool: &SevenZipTool, archive: &Path) -> Result<Vec<ListedEntry>> {
    let output = command(tool, "l").arg("-slt").arg(archive).output()?;
    if !output.status.success() {
        return Err(tool_error(tool, "archive_7z_list_failed", &output.stderr));
    }
    parse_listing(&String::from_utf8_lossy(&output.stdout))
}

/// Runs `cmd` to its end, killing it once `cancelled` says so. Returns
/// its exit status and what it wrote to stderr.
fn run(
    mut cmd: Command,
    cancelled: &dyn Fn() -> bool,
) -> Result<(std::process::ExitStatus, Vec<u8>)> {
    let mut child = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
    // Drained concurrently: an unread stderr pipe fills up (64 KiB) and
    // blocks 7z on its next warning, so it would never end.
    let stderr = drain_stderr(child.stderr.take());
    let status = loop {
        if cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(crate::fs::error::cancelled());
        }
        match child.try_wait()? {
            Some(status) => break status,
            None => std::thread::sleep(std::time::Duration::from_millis(30)),
        }
    };
    Ok((status, stderr.join().unwrap_or_default()))
}

impl External7zReader {
    /// Unpacks `chosen` (all entries when `everything`) into `out`.
    fn unpack(
        tool: &SevenZipTool,
        archive: &Path,
        chosen: &[&ListedEntry],
        everything: bool,
        out: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<()> {
        let mut cmd = command(tool, "x");
        cmd.arg("-bso0")
            .arg("-bsp0")
            .arg(format!("-o{}", out.display()));
        // Kept until 7z is done with it.
        let mut names = None;
        if !everything {
            // Names are taken literally (`-spd`: `[`, `*` are no wildcards),
            // one per line, so no command line grows too long.
            let mut file = tempfile::Builder::new()
                .prefix("pairee_7z_names_")
                .tempfile()?;
            for entry in chosen {
                writeln!(file, "{}", entry.path)?;
            }
            file.flush()?;
            // Closed (Windows does not let 7z open it while it is open
            // here); removed once dropped.
            names = Some(file.into_temp_path());
        }
        // Archive paths are absolute: never taken for a switch.
        cmd.arg(archive);
        if let Some(file) = &names {
            cmd.arg("-spd")
                .arg("-scsUTF-8")
                .arg(format!("@{}", file.display()));
        }
        let (status, stderr) = run(cmd, cancelled)?;
        drop(names);
        match status.code() {
            // 1: warnings (a file could not be read); what is missing
            // shows up when it is opened.
            Some(0 | 1) => Ok(()),
            _ => Err(tool_error(tool, "archive_7z_extract_failed", &stderr)),
        }
    }
}

impl ArchiveReader for External7zReader {
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>> {
        let tool = tool()?;
        Ok(list(&tool, archive)?
            .iter()
            .map(ListedEntry::meta)
            .collect())
    }

    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()> {
        self.visit_matching(archive, &|_| true, &|| false, visit)
    }

    fn visit_matching(
        &self,
        archive: &Path,
        wanted: &dyn Fn(&str) -> bool,
        cancelled: &dyn Fn() -> bool,
        visit: &mut Visitor,
    ) -> Result<()> {
        let tool = tool()?;
        let entries = list(&tool, archive)?;
        let chosen: Vec<&ListedEntry> = entries.iter().filter(|e| wanted(&e.path)).collect();
        if chosen.is_empty() {
            return Ok(());
        }
        let scratch = tempfile::Builder::new().prefix("pairee_7z_").tempdir()?;
        let out = scratch.path();
        // Fail closed: nothing is unpacked unless every entry is safe.
        preflight_listing(&chosen, &mut ExtractGuard::new(out)?)?;
        let everything = chosen.len() == entries.len();
        Self::unpack(&tool, archive, &chosen, everything, out, cancelled)?;

        for entry in chosen {
            let rel = ExtractGuard::sanitize(&entry.path)?;
            let meta = entry.meta();
            let next = if entry.is_dir {
                visit(&meta, &mut std::io::empty())?
            } else {
                let mut file = std::fs::File::open(out.join(&rel)).map_err(|e| {
                    anyhow!(t_fill(
                        "archive_7z_entry_missing",
                        &[&entry.path, &e.to_string()]
                    ))
                })?;
                visit(&meta, &mut file)?
            };
            if next == Visit::Stop {
                break;
            }
        }
        Ok(())
    }

    fn count_hint(&self, archive: &Path) -> usize {
        tool()
            .and_then(|tool| list(&tool, archive))
            .map_or(0, |entries| entries.len())
    }

    fn test(&self, archive: &Path) -> Result<()> {
        let tool = tool()?;
        let mut cmd = command(&tool, "t");
        cmd.arg("-bso0").arg("-bsp0").arg(archive);
        let (status, stderr) = run(cmd, &|| false)?;
        if status.success() {
            return Ok(());
        }
        Err(tool_error(&tool, "archive_7z_test_failed", &stderr))
    }
}

/// Reads the child's stderr to its end on its own thread (capped so a
/// chatty tool cannot grow memory).
fn drain_stderr(pipe: Option<std::process::ChildStderr>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else {
            return Vec::new();
        };
        let mut text = Vec::new();
        let _ = (&mut pipe).take(16 * 1024).read_to_end(&mut text);
        // Keep consuming so the child never blocks on a full pipe.
        let _ = std::io::copy(&mut pipe, &mut std::io::sink());
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "7-Zip 26.01\r\n\r\nListing archive: C:\\a.rar\r\n\r\n--\r\nPath = C:\\a.rar\r\nType = Rar\r\n\r\n----------\r\nPath = docs\r\nFolder = +\r\nSize = \r\nModified = 2026-10-18 08:40:49.4852309\r\n\r\nPath = docs\\readme.txt\r\nFolder = -\r\nSize = 12\r\n\r\nPath = docs\\readme.txt:Zone.Identifier\r\nSize = 3\r\nAlternate Stream = +\r\n\r\n";

    fn entry(path: &str, size: u64, is_dir: bool) -> ListedEntry {
        ListedEntry {
            path: path.into(),
            size,
            is_dir,
            modified: None,
        }
    }

    #[test]
    fn parse_listing_skips_archive_header_and_streams() {
        let entries = parse_listing(LISTING).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "docs");
        assert!(entries[0].is_dir);
        assert!(entries[0].modified.is_some());
        assert_eq!(entries[1], entry("docs\\readme.txt", 12, false));
    }

    #[test]
    fn parse_listing_fails_closed_on_garbage() {
        assert!(parse_listing("error: cannot open").is_err());
    }

    #[test]
    fn preflight_rejects_traversal_nesting_and_bombs() {
        let dir = tempfile::tempdir().unwrap();
        let check = |entries: &[ListedEntry], guard: &mut ExtractGuard| {
            preflight_listing(&entries.iter().collect::<Vec<_>>(), guard)
        };

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        assert!(check(&[entry("../x", 1, false)], &mut guard).is_err());

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        let nested = [entry("link", 0, false), entry("link/pwn", 1, false)];
        assert!(check(&nested, &mut guard).is_err());

        let mut guard = ExtractGuard::with_limits(dir.path(), 100, 10).unwrap();
        assert!(check(&[entry("big", 101, false)], &mut guard).is_err());

        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        let ok = [entry("docs", 0, true), entry("docs/a.txt", 5, false)];
        assert!(check(&ok, &mut guard).is_ok());
    }
}
