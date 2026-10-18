//! Extraction shared by every native format: entries stream from an
//! [`ArchiveReader`] and every write goes through the [`ExtractGuard`]
//! (no traversal, no writing through links, no overwrite, size and entry
//! limits) — unless the user chose to replace an existing file.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit};
use super::safe_extract::ExtractGuard;
use crate::fs::progress::ArchiveReport;

/// Maps an entry's sanitized relative path to where it is written below
/// the destination, or `None` to leave it out.
pub type Selector<'a> = dyn Fn(&Path) -> Option<PathBuf> + 'a;

/// What to do with files whose destination already exists, by their path
/// below the destination (the answers to the job's conflict setting).
/// Files in none of the sets are never overwritten: they are skipped.
#[derive(Debug, Default, Clone)]
pub struct Existing {
    /// Left where they are (reported as skipped).
    pub skip: HashSet<PathBuf>,
    /// Written under another name.
    pub rename: HashMap<PathBuf, PathBuf>,
    /// The existing file is replaced.
    pub replace: HashSet<PathBuf>,
}

/// The selected, non-empty destination path of the entry named `name`.
fn chosen(select: &Selector, name: &str) -> Option<PathBuf> {
    let rel = ExtractGuard::sanitize(name).ok()?;
    select(&rel).filter(|r| !r.as_os_str().is_empty())
}

/// Extracts the entries of `archive` chosen by `select` into `dest`.
/// Entries that are never materialised are reported as skipped.
pub fn extract_entries(
    reader: &dyn ArchiveReader,
    archive: &Path,
    dest: &Path,
    select: &Selector,
    existing: &Existing,
    report: &mut ArchiveReport,
) -> Result<()> {
    let mut guard = ExtractGuard::new(dest)?;
    guard.replace_existing(existing.replace.iter().cloned());
    let total_files = reader.count_hint(archive);
    let mut files_copied = 0usize;
    let cancel = report.cancel_flag();
    let cancelled = || cancel.load(std::sync::atomic::Ordering::Relaxed);
    // Unsafe names are not filtered out here: the visitor rejects them.
    let wanted =
        |name: &str| ExtractGuard::sanitize(name).is_err() || chosen(select, name).is_some();
    reader.visit_matching(archive, &wanted, &cancelled, &mut |meta, data| {
        report.ensure_running()?;
        guard.count_entry()?;
        let rel = ExtractGuard::sanitize(&meta.name)?;
        let Some(rel) = select(&rel).filter(|r| !r.as_os_str().is_empty()) else {
            return Ok(Visit::Continue);
        };
        if existing.skip.contains(&rel) {
            report.skipped(&rel);
            return Ok(Visit::Continue);
        }
        let rel = existing.rename.get(&rel).cloned().unwrap_or(rel);
        report.entry(&crate::fs::file_name_lossy(&rel), files_copied, total_files);
        files_copied += 1;
        if !write_entry(&mut guard, &rel, meta, data, &cancelled)? {
            // Reported, so a move out of the archive keeps the entry.
            report.unsupported(&rel);
        }
        Ok(Visit::Continue)
    })?;
    report.ensure_running()?;
    guard.report_skipped(report);
    Ok(())
}

/// Writes one entry; `false` when its kind is never materialised.
fn write_entry(
    guard: &mut ExtractGuard,
    rel: &Path,
    meta: &EntryMeta,
    data: &mut dyn Read,
    cancelled: &dyn Fn() -> bool,
) -> Result<bool> {
    match &meta.kind {
        EntryKind::Dir => {
            guard.create_dir(rel)?;
        }
        EntryKind::File => {
            guard.check_declared_size(meta.size)?;
            #[cfg_attr(not(unix), allow(unused_variables))]
            let written = guard.write_file(rel, data, cancelled)?;
            #[cfg(unix)]
            if let (Some(out), Some(mode)) = (written, meta.mode) {
                use std::os::unix::fs::PermissionsExt;
                // Drop setuid/setgid/sticky bits from archive modes.
                let _ = out.set_permissions(std::fs::Permissions::from_mode(mode & 0o777));
            }
        }
        #[cfg(unix)]
        EntryKind::Symlink(target) => guard.create_symlink(rel, target)?,
        other => {
            // Hard links, devices, FIFOs (and symlinks on Windows) are not
            // materialised: they can alias files outside the destination.
            tracing::warn!(
                "extract: skipping unsupported entry {:?} ({:?})",
                rel,
                other
            );
            return Ok(false);
        }
    }
    Ok(true)
}
