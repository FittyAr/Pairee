//! Zip format: reading entries (Strategy for [`ArchiveReader`]) and
//! compressing files and folders into a new archive.

use anyhow::Result;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use zip::ZipArchive;

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit, Visitor, unix_time};
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

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

pub fn compress_zip(
    sources: Vec<PathBuf>,
    dest_archive: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let file = fs::File::create(dest_archive)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut i = 0usize;
    let total_files = sources.len();

    for src in sources {
        ensure_not_cancelled(cancel)?;
        if src.is_dir() {
            let top = src.file_name().unwrap_or_default().to_os_string();
            let base_parent = src.parent().map(|p| p.to_path_buf());
            let mut stack: Vec<PathBuf> = vec![src.clone()];
            while let Some(dir) = stack.pop() {
                ensure_not_cancelled(cancel)?;
                let rel_dir = dir.strip_prefix(&src).unwrap_or(Path::new(""));
                zip.add_directory(zip_entry_name(&top, rel_dir), options)?;

                let entries = match fs::read_dir(&dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                for entry in entries.flatten() {
                    ensure_not_cancelled(cancel)?;
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        stack.push(entry_path);
                    } else {
                        let rel = entry_path.strip_prefix(&src).unwrap_or(&entry_path);
                        let zip_path_str = zip_entry_name(&top, rel);
                        let _ = tx.blocking_send(ProgressUpdate {
                            skipped: false,
                            current_file: zip_path_str.clone(),
                            files_copied: i,
                            total_files,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: None,
                        });
                        zip.start_file(zip_path_str, options)?;
                        let mut f = match fs::File::open(&entry_path) {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("compress_zip: skipping {:?}: {}", entry_path, e);
                                continue;
                            }
                        };
                        io::copy(&mut f, &mut zip)?;
                        i += 1;
                    }
                }
            }
            let _ = base_parent;
        } else {
            let name = src.file_name().unwrap_or_default().to_string_lossy();
            let _ = tx.blocking_send(ProgressUpdate {
                skipped: false,
                current_file: name.to_string(),
                files_copied: i,
                total_files,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });

            zip.start_file(name, options)?;
            let mut f = fs::File::open(&src)?;
            io::copy(&mut f, &mut zip)?;
            i += 1;
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

        let (tx, _rx) = mpsc::channel(64);
        extract_archive(&archive, &dest, &tx, &AtomicBool::new(false)).unwrap();
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

        let (tx, _rx) = mpsc::channel(64);
        assert!(extract_archive(&archive, &dest, &tx, &AtomicBool::new(false)).is_err());
        assert!(!dir.path().join("evil.txt").exists());
    }
}
