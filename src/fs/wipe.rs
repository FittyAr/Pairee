use anyhow::{Context, Result};
use std::fs::{File, Metadata};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

/// Number of overwrite passes used for the wipe operation.
const WIPE_PASSES: usize = 3;

/// Size of the pattern buffer written per `write_all` call.
const WIPE_CHUNK: usize = 65536;

/// Securely overwrites the file with random-like byte patterns across multiple passes,
/// then truncates it to zero and removes it from the filesystem.
///
/// Symbolic links (and Windows junctions) are never followed: wiping a link
/// removes only the link itself and leaves its target untouched.
///
/// This makes content recovery significantly harder, though not cryptographically
/// guaranteed on SSDs with wear-leveling.
pub fn wipe_file(path: &Path) -> Result<()> {
    let link_meta = std::fs::symlink_metadata(path)
        .with_context(|| format!("Reading metadata for wipe: {:?}", path))?;

    if link_meta.file_type().is_symlink() {
        remove_link(path, &link_meta)?;
        remove_description(path);
        return Ok(());
    }

    if link_meta.is_dir() {
        anyhow::bail!("wipe_file cannot wipe a directory: {:?}", path);
    }

    {
        let mut file = open_no_follow(path)?;
        // Re-check on the opened handle: the path may have been swapped
        // between `symlink_metadata` and `open`.
        let meta = file.metadata().context("Reading metadata of opened file")?;
        if !meta.is_file() || !same_file(&link_meta, &meta) {
            anyhow::bail!(
                "wipe_file target changed or is not a regular file: {:?}",
                path
            );
        }

        let file_size = meta.len();
        if file_size > 0 {
            // Overwrite with alternating patterns (0x00, 0xFF, 0x55)
            let patterns: &[u8] = &[0x00, 0xFF, 0x55];
            for pass in 0..WIPE_PASSES {
                overwrite_with_byte(&mut file, file_size, patterns[pass % patterns.len()])?;
            }

            // Final pass: zero-fill and truncate
            overwrite_with_byte(&mut file, file_size, 0)?;
            file.set_len(0).context("Truncating wiped file")?;
        }
    }

    remove_description(path);
    std::fs::remove_file(path).with_context(|| format!("Removing file after wipe: {:?}", path))
}

fn remove_description(path: &Path) {
    if let (Some(parent), Some(filename)) = (path.parent(), path.file_name())
        && let Some(filename_str) = filename.to_str()
    {
        let _ = crate::fs::descriptions::remove_description(parent, filename_str);
    }
}

/// Removes a symlink/junction without touching what it points to.
fn remove_link(path: &Path, meta: &Metadata) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if meta.file_type().is_symlink_dir() {
            return std::fs::remove_dir(path)
                .with_context(|| format!("Removing directory link: {:?}", path));
        }
    }
    #[cfg(not(windows))]
    let _ = meta;
    std::fs::remove_file(path).with_context(|| format!("Removing link: {:?}", path))
}

/// Opens `path` for writing without following a symlink in the final component.
fn open_no_follow(path: &Path) -> Result<File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    opts.open(path)
        .with_context(|| format!("Opening file for wipe: {:?}", path))
}

#[cfg(unix)]
fn same_file(a: &Metadata, b: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(_a: &Metadata, b: &Metadata) -> bool {
    // FILE_FLAG_OPEN_REPARSE_POINT already guarantees the handle is not a
    // followed link; a swapped-in link would surface as non-regular here.
    !b.file_type().is_symlink()
}

/// Overwrites the entire contents of the open `file` with `byte`.
fn overwrite_with_byte(file: &mut File, file_size: u64, byte: u8) -> Result<()> {
    let chunk = vec![byte; (file_size as usize).min(WIPE_CHUNK)];
    file.seek(SeekFrom::Start(0))
        .context("Seeking for wipe pass")?;

    let mut written: u64 = 0;
    while written < file_size {
        let to_write = ((file_size - written) as usize).min(chunk.len());
        file.write_all(&chunk[..to_write])
            .context("Writing wipe data")?;
        written += to_write as u64;
    }
    file.flush().context("Flushing wipe pass")?;
    // Force the kernel to flush the wipe data to disk before we move on to
    // the next pass. Without this, the OS may keep the overwrite in the page
    // cache and later writes (or even the unlink) could return success
    // before the bytes are actually on the storage, defeating the wipe.
    file.sync_all().context("Syncing wipe pass to disk")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_wipe_file_removes_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("secret.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"sensitive data").unwrap();
        drop(f);

        assert!(path.exists());
        wipe_file(&path).expect("wipe should succeed");
        assert!(!path.exists(), "File should be removed after wipe");
    }

    #[test]
    fn test_wipe_empty_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("empty.txt");
        std::fs::File::create(&path).unwrap();

        wipe_file(&path).expect("wipe of empty file should succeed");
        assert!(!path.exists());
    }

    #[test]
    fn test_wipe_symlink_removes_only_link() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target.txt");
        std::fs::write(&target, b"keep me").unwrap();
        let link = dir.path().join("link.txt");

        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&target, &link);
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_file(&target, &link);
        if created.is_err() {
            // Symlink creation needs Developer Mode / admin on Windows.
            return;
        }

        wipe_file(&link).expect("wipe of link should succeed");
        assert!(std::fs::symlink_metadata(&link).is_err(), "link removed");
        assert_eq!(std::fs::read(&target).unwrap(), b"keep me");
    }
}
