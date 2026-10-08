//! Tar and Tar.gz extraction operations.

use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tar::{Archive, EntryType};
use tokio::sync::mpsc;

use super::safe_extract::ExtractGuard;
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};

pub fn extract_tar_gz(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let tar_gz = fs::File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    let mut guard = ExtractGuard::new(dest_dir)?;

    for (i, entry) in archive.entries()?.enumerate() {
        ensure_not_cancelled(cancel)?;
        guard.count_entry()?;
        let mut file = entry?;
        let rel = ExtractGuard::sanitize(&file.path()?.to_string_lossy())?;
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
            total_files: 0, // Unknown without pre-scanning
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        match file.header().entry_type() {
            EntryType::Directory => {
                guard.create_dir(&rel)?;
            }
            EntryType::Regular | EntryType::Continuous => {
                guard.check_declared_size(file.size())?;
                if let Some(mut out) = guard.create_file(&rel)? {
                    guard.copy_limited(&mut file, &mut out)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        // Drop setuid/setgid/sticky bits from archive modes.
                        if let Ok(mode) = file.header().mode() {
                            let _ = out.set_permissions(fs::Permissions::from_mode(mode & 0o777));
                        }
                    }
                }
            }
            #[cfg(unix)]
            EntryType::Symlink => {
                if let Some(target) = file.link_name()? {
                    guard.create_symlink(&rel, &target)?;
                }
            }
            other => {
                // Hard links, devices, FIFOs (and symlinks on Windows) are
                // not materialised: they can alias files outside `dest_dir`.
                tracing::warn!(
                    "extract: skipping unsupported tar entry {:?} ({:?})",
                    rel,
                    other
                );
            }
        }
    }

    guard.report_skipped(tx);
    Ok(())
}

pub fn list_tar_gz_files(path: &Path) -> Result<Vec<String>> {
    let tar_gz = fs::File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    let mut list = Vec::new();
    for entry in archive.entries()?.flatten() {
        if let Ok(path) = entry.path() {
            list.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_tar_gz(path: &Path, entries: &[(&str, &[u8])]) {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, body) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, *body).unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar_bytes).unwrap();
        fs::write(path, gz.finish().unwrap()).unwrap();
    }

    #[test]
    fn extract_does_not_overwrite_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        write_tar_gz(&archive, &[("keep.txt", b"new"), ("sub/new.txt", b"hello")]);
        let dest = dir.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), b"original").unwrap();

        let (tx, _rx) = mpsc::channel(64);
        extract_tar_gz(&archive, &dest, &tx, &AtomicBool::new(false)).unwrap();
        assert_eq!(fs::read(dest.join("keep.txt")).unwrap(), b"original");
        assert_eq!(
            fs::read(dest.join("sub").join("new.txt")).unwrap(),
            b"hello"
        );
    }
}
