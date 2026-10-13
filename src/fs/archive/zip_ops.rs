//! Zip format: reading entries (Strategy for [`ArchiveReader`]) and
//! compressing files and folders into a new archive.

use anyhow::Result;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit, Visitor, unix_time};
use super::zip_write::file_options;
use crate::fs::progress::ArchiveReport;
use crate::fs::vfs::LocalVfs;
use crate::fs::walk::{WalkOptions, Walker};

pub struct ZipReader;

fn open(archive: &Path) -> Result<ZipArchive<fs::File>> {
    Ok(ZipArchive::new(fs::File::open(archive)?)?)
}

/// Zip times carry no zone; they are read as UTC.
fn zip_time(dt: zip::DateTime) -> Option<std::time::SystemTime> {
    let date =
        chrono::NaiveDate::from_ymd_opt(dt.year().into(), dt.month().into(), dt.day().into())?;
    let time = date.and_hms_opt(dt.hour().into(), dt.minute().into(), dt.second().into())?;
    u64::try_from(time.and_utc().timestamp())
        .ok()
        .map(unix_time)
}

fn meta<R: Read>(file: &zip::read::ZipFile<'_, R>) -> EntryMeta {
    EntryMeta {
        name: file.name().to_string(),
        // Links are stored as small files holding the target; they are
        // extracted as such, never as links.
        kind: if file.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        },
        size: file.size(),
        modified: file.last_modified().and_then(zip_time),
        mode: None,
    }
}

impl ArchiveReader for ZipReader {
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>> {
        let mut zip = open(archive)?;
        (0..zip.len())
            .map(|i| Ok(meta(&zip.by_index_raw(i)?)))
            .collect()
    }

    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()> {
        let mut zip = open(archive)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if visit(&meta(&file), &mut file)? == Visit::Stop {
                break;
            }
        }
        Ok(())
    }

    fn count_hint(&self, archive: &Path) -> usize {
        open(archive).map_or(0, |zip| zip.len())
    }

    fn writable(&self) -> bool {
        true
    }
}

/// Packs `sources` (files, and folders with everything below them; links
/// are stored as what they point to, links to folders as empty folders)
/// into the new archive `dest_archive`. An unreadable file or folder fails
/// the compression instead of leaving it out.
pub fn compress_zip(
    sources: Vec<PathBuf>,
    dest_archive: &Path,
    report: &mut ArchiveReport,
) -> Result<()> {
    let mut zip = zip::ZipWriter::new(fs::File::create(dest_archive)?);
    let total_files = sources.len();
    let mut added = 0usize;
    let mut add_file = |zip: &mut zip::ZipWriter<fs::File>,
                        report: &mut ArchiveReport,
                        path: &Path,
                        name: String|
     -> Result<()> {
        report.entry(&name, added, total_files);
        let mut file = fs::File::open(path)?;
        zip.start_file(name, file_options(file.metadata()?.len()))?;
        io::copy(&mut file, zip)?;
        added += 1;
        Ok(())
    };
    let cancel = report.cancel_flag();
    let cancelled = || cancel.load(std::sync::atomic::Ordering::Relaxed);
    for src in sources {
        report.ensure_running()?;
        let top = src.file_name().unwrap_or_default().to_os_string();
        if !src.is_dir() {
            add_file(&mut zip, report, &src, top.to_string_lossy().into_owned())?;
            continue;
        }
        zip.add_directory(zip_entry_name(&top, Path::new("")), file_options(0))?;
        for item in Walker::new(&LocalVfs, &src, WalkOptions::default()).with_cancel(&cancelled) {
            let item = item?;
            let rel = item.path().strip_prefix(&src).unwrap_or(item.path());
            let name = zip_entry_name(&top, rel);
            if item.entry.is_dir {
                zip.add_directory(name, file_options(0))?;
            } else {
                add_file(&mut zip, report, item.path(), name)?;
            }
        }
    }
    zip.finish()?;
    Ok(())
}

/// Zip entry name for `rel` (relative to a compressed folder) below that
/// folder's name `top`. Zip paths always use `/`, whatever the host OS.
fn zip_entry_name(top: &std::ffi::OsStr, rel: &Path) -> String {
    std::iter::once(top)
        .chain(rel.components().map(|c| c.as_os_str()))
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::archive::extract_archive;
    use crate::fs::archive::test_fixtures::write_zip;

    #[test]
    fn extract_does_not_overwrite_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        write_zip(&archive, &[("keep.txt", b"new"), ("sub/new.txt", b"hello")]);
        let dest = dir.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), b"original").unwrap();

        extract_archive(&archive, &dest, &mut ArchiveReport::detached(false)).unwrap();
        assert_eq!(fs::read(dest.join("keep.txt")).unwrap(), b"original");
        assert_eq!(
            fs::read(dest.join("sub").join("new.txt")).unwrap(),
            b"hello"
        );
    }

    #[test]
    fn extract_rejects_traversal_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.zip");
        write_zip(&archive, &[("../evil.txt", b"x")]);
        let dest = dir.path().join("out");

        assert!(extract_archive(&archive, &dest, &mut ArchiveReport::detached(false)).is_err());
        assert!(!dir.path().join("evil.txt").exists());
    }
}
