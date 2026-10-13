//! Extraction shared by every native format: entries stream from an
//! [`ArchiveReader`] and every write goes through the [`ExtractGuard`]
//! (no traversal, no writing through links, no overwrite, size and entry
//! limits).

use anyhow::Result;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit};
use super::safe_extract::ExtractGuard;
use crate::fs::progress::ArchiveReport;

/// Maps an entry's sanitized relative path to where it is written below
/// the destination, or `None` to leave it out.
pub type Selector<'a> = dyn Fn(&Path) -> Option<PathBuf> + 'a;

/// Extracts the entries of `archive` chosen by `select` into `dest`.
pub fn extract_entries(
    reader: &dyn ArchiveReader,
    archive: &Path,
    dest: &Path,
    select: &Selector,
    report: &mut ArchiveReport,
) -> Result<()> {
    let mut guard = ExtractGuard::new(dest)?;
    let total_files = reader.count_hint(archive);
    let mut files_copied = 0usize;
    reader.visit(archive, &mut |meta, data| {
        report.ensure_running()?;
        guard.count_entry()?;
        let rel = ExtractGuard::sanitize(&meta.name)?;
        let Some(rel) = select(&rel).filter(|r| !r.as_os_str().is_empty()) else {
            return Ok(Visit::Continue);
        };
        report.entry(&crate::fs::file_name_lossy(&rel), files_copied, total_files);
        files_copied += 1;
        write_entry(&mut guard, &rel, meta, data)?;
        Ok(Visit::Continue)
    })?;
    report.ensure_running()?;
    guard.report_skipped(report);
    Ok(())
}

/// Every entry at its own relative path.
pub fn everything(rel: &Path) -> Option<PathBuf> {
    Some(rel.to_path_buf())
}

fn write_entry(
    guard: &mut ExtractGuard,
    rel: &Path,
    meta: &EntryMeta,
    data: &mut dyn Read,
) -> Result<()> {
    match &meta.kind {
        EntryKind::Dir => {
            guard.create_dir(rel)?;
        }
        EntryKind::File => {
            guard.check_declared_size(meta.size)?;
            if let Some(mut out) = guard.create_file(rel)? {
                guard.copy_limited(data, &mut out)?;
                #[cfg(unix)]
                if let Some(mode) = meta.mode {
                    use std::os::unix::fs::PermissionsExt;
                    // Drop setuid/setgid/sticky bits from archive modes.
                    let _ = out.set_permissions(std::fs::Permissions::from_mode(mode & 0o777));
                }
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
        }
    }
    Ok(())
}
