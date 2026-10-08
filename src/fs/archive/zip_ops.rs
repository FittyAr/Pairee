//! Zip extraction and compression operations.

use anyhow::Result;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use zip::ZipArchive;

use super::safe_extract::ExtractGuard;
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_zip(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let total_files = archive.len();

    let mut guard = ExtractGuard::new(dest_dir)?;

    for i in 0..total_files {
        ensure_not_cancelled(cancel)?;
        guard.count_entry()?;
        let mut file = archive.by_index(i)?;
        let rel = ExtractGuard::sanitize(file.name())?;
        if rel.as_os_str().is_empty() {
            continue;
        }

        let file_name = rel
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let _ = tx.blocking_send(ProgressUpdate {
            skipped: false,
            current_file: file_name,
            files_copied: i,
            total_files,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        if file.is_dir() {
            guard.create_dir(&rel)?;
        } else {
            guard.check_declared_size(file.size())?;
            if let Some(mut outfile) = guard.create_file(&rel)? {
                guard.copy_limited(&mut file, &mut outfile)?;
            }
        }
    }

    guard.report_skipped(tx);
    Ok(())
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
                let dir_name_in_zip = match dir.strip_prefix(&src) {
                    Ok(rel) if !rel.as_os_str().is_empty() => {
                        let mut p = top.clone();
                        for component in rel.components() {
                            p.push(component.as_os_str());
                        }
                        p
                    }
                    _ => top.clone(),
                };
                zip.add_directory(dir_name_in_zip.to_string_lossy(), options)?;

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
                        let mut zip_path = top.clone();
                        for component in rel.components() {
                            zip_path.push(component.as_os_str());
                        }
                        let zip_path_str = zip_path.to_string_lossy().into_owned();
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

pub fn list_zip_files(path: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut list = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            list.push(file.name().to_string());
        }
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, body) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_does_not_overwrite_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        write_zip(&archive, &[("keep.txt", b"new"), ("sub/new.txt", b"hello")]);
        let dest = dir.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), b"original").unwrap();

        let (tx, _rx) = mpsc::channel(64);
        extract_zip(&archive, &dest, &tx, &AtomicBool::new(false)).unwrap();
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
        assert!(extract_zip(&archive, &dest, &tx, &AtomicBool::new(false)).is_err());
        assert!(!dir.path().join("evil.txt").exists());
    }
}
