//! The second half of a move into or out of an archive: once the copy is
//! done, what was copied is removed from where it came from — and only
//! that: an entry the extraction skipped (something was in the way) stays
//! in the archive, a source the conflict setting left out stays on disk.

use super::archive_vfs::{Placed, apply_edit};
use crate::config::localization::t;
use crate::fs::archive::{ArchiveEdit, ArchiveVfs, EntrySource, Selection};
use crate::fs::progress::ArchiveReport;
use crate::fs::vfs::Vfs;
use std::path::Path;

/// The archive can lose entries (moving out of it is possible).
pub fn editable(archive: &Path) -> bool {
    ArchiveVfs::open(archive.to_path_buf()).is_some_and(|vfs| vfs.capabilities().remove)
}

/// Removes from `archive` the entries `selection` took that were
/// extracted in full: the selected items, or only the file entries
/// `taken` when a filter left others out.
pub fn remove_extracted(
    archive: &Path,
    selection: &Selection,
    taken: Option<&Vec<Placed>>,
    report: &mut ArchiveReport,
) -> anyhow::Result<()> {
    let edit = extracted_removal(selection, taken, report);
    if edit.remove.is_empty() {
        return Ok(());
    }
    apply_edit(archive, &edit, report)
}

/// The candidates none of whose files was skipped or failed (they are
/// reported by where they were going).
fn extracted_removal(
    selection: &Selection,
    taken: Option<&Vec<Placed>>,
    report: &ArchiveReport,
) -> ArchiveEdit {
    let results = report.results();
    let left: Vec<&Path> = results
        .skipped_files
        .iter()
        .map(|s| s.src.as_path())
        .chain(results.failed_files.iter().map(|f| f.src.as_path()))
        .collect();
    let candidates: Vec<Placed> = taken.cloned().unwrap_or_else(|| {
        selection
            .items
            .iter()
            .map(|item| {
                let rel = selection.place(item).unwrap_or_else(|| item.to_path_buf());
                (item.clone(), rel)
            })
            .collect()
    });
    let remove = candidates
        .into_iter()
        .filter(|(_, rel)| !left.iter().any(|l| l.starts_with(rel)))
        .map(|(inner, _)| inner)
        .collect();
    ArchiveEdit {
        remove,
        ..ArchiveEdit::default()
    }
}

/// Deletes the local files `edit` added to the archive, then their folders
/// when that leaves them empty (a folder with a file the conflict setting
/// skipped stays). A file that cannot be deleted is reported as failed.
pub fn delete_imported(edit: &ArchiveEdit, report: &mut ArchiveReport) {
    for (_, source) in edit.add.iter().rev() {
        let EntrySource::File(path) = source else {
            continue;
        };
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir()) {
            let _ = std::fs::remove_dir(path);
        } else if let Err(e) = crate::fs::remove::remove_entry(path) {
            let error = t("error_delete_source_failed")
                .replacen("{}", &path.to_string_lossy(), 1)
                .replacen("{}", &e.to_string(), 1);
            report.failed(path, error);
        }
    }
}
