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
use crate::fs::transfer::filter::TransferFilter;

/// Which entries an extraction takes and where they go: `items` (paths
/// inside the archive, files or folders with everything below them; the
/// empty path is the whole archive) relative to the archive folder `base`
/// (the folder the panel shows), the one item under another name when
/// `rename_to` is set, and only the entries the job's `filter` keeps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub base: PathBuf,
    pub items: Vec<PathBuf>,
    pub rename_to: Option<std::ffi::OsString>,
    pub filter: TransferFilter,
}

impl Selection {
    /// Every entry of the archive.
    pub fn whole() -> Self {
        Self {
            items: vec![PathBuf::new()],
            ..Self::default()
        }
    }

    /// Where the entry at `inner` goes below the destination, `None` when
    /// it is not selected.
    pub fn place(&self, inner: &Path) -> Option<PathBuf> {
        if !self.items.iter().any(|item| inner.starts_with(item)) {
            return None;
        }
        let rel = inner.strip_prefix(&self.base).ok()?;
        let Some(name) = &self.rename_to else {
            return Some(rel.to_path_buf());
        };
        let mut parts = rel.components();
        parts.next()?;
        Some(Path::new(name).join(parts.as_path()))
    }

    /// Where the entry `meta` (stored at `inner`) goes below the
    /// destination, `None` when it is not selected or the filter leaves it
    /// out. As in a copy between folders, the filter excludes folders
    /// below the selected items by name and applies in full to files.
    pub fn take(&self, inner: &Path, meta: &EntryMeta) -> Option<PathBuf> {
        let rel = self.place(inner)?;
        if self.filter.rules.is_empty() {
            return Some(rel);
        }
        let below = self
            .items
            .iter()
            .filter_map(|item| inner.strip_prefix(item).ok())
            .min_by_key(|below| below.components().count())?;
        let kept = match meta.kind {
            EntryKind::Dir => !self.filter.excludes_folder_in(below),
            _ => {
                !self
                    .filter
                    .excludes_folder_in(below.parent().unwrap_or(Path::new("")))
                    && self.filter.matches(inner, meta.size, meta.modified)
            }
        };
        kept.then_some(rel)
    }

    /// Every entry, unfiltered: the archive's own entry count is the
    /// number of entries extracted.
    fn is_whole(&self) -> bool {
        *self == Self::whole()
    }
}

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

/// Extracts the entries of `archive` that `selection` takes into `dest`.
/// Entries that are never materialised are reported as skipped. The
/// progress total is the archive's entry count for a whole extraction;
/// otherwise the job announces its own.
pub fn extract_entries(
    reader: &dyn ArchiveReader,
    archive: &Path,
    dest: &Path,
    selection: &Selection,
    existing: &Existing,
    report: &mut ArchiveReport,
) -> Result<()> {
    let mut guard = ExtractGuard::new(dest)?;
    guard.replace_existing(existing.replace.iter().cloned());
    let total_files = if selection.is_whole() {
        reader.count_hint(archive)
    } else {
        0
    };
    let mut files_copied = 0usize;
    let cancel = report.cancel_flag();
    let cancelled = || cancel.load(std::sync::atomic::Ordering::Relaxed);
    // Unsafe names are not filtered out here: the visitor rejects them.
    let wanted = |name: &str| {
        ExtractGuard::sanitize(name).map_or(true, |rel| {
            selection
                .place(&rel)
                .is_some_and(|r| !r.as_os_str().is_empty())
        })
    };
    reader.visit_matching(archive, &wanted, &cancelled, &mut |meta, data| {
        report.ensure_running()?;
        guard.count_entry()?;
        let inner = ExtractGuard::sanitize(&meta.name)?;
        let Some(rel) = selection
            .take(&inner, meta)
            .filter(|r| !r.as_os_str().is_empty())
        else {
            return Ok(Visit::Continue);
        };
        if existing.skip.contains(&rel) {
            report.skipped(&rel);
            return Ok(Visit::Continue);
        }
        let rel = existing.rename.get(&rel).cloned().unwrap_or(rel);
        report.entry(
            &archive.join(&inner),
            dest.join(&rel),
            files_copied,
            total_files,
        );
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
