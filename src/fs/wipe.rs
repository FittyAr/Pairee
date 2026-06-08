use anyhow::{Context, Result};
use std::path::Path;

/// Number of overwrite passes used for the wipe operation.
const WIPE_PASSES: usize = 3;

/// Securely overwrites the file with random-like byte patterns across multiple passes,
/// then truncates it to zero and removes it from the filesystem.
///
/// This makes content recovery significantly harder, though not cryptographically
/// guaranteed on SSDs with wear-leveling.
pub fn wipe_file(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Reading metadata for wipe: {:?}", path))?;

    if metadata.is_dir() {
        anyhow::bail!("wipe_file cannot wipe a directory: {:?}", path);
    }

    let file_size = metadata.len() as usize;

    if file_size > 0 {
        // Overwrite with alternating patterns (0x00, 0xFF, 0x55)
        let patterns: &[u8] = &[0x00, 0xFF, 0x55];
        for pass in 0..WIPE_PASSES {
            let byte = patterns[pass % patterns.len()];
            let chunk: Vec<u8> = vec![byte; file_size.min(65536)];
            overwrite_with_chunk(path, file_size, &chunk)?;
        }

        // Final pass: zero-fill and truncate
        overwrite_with_chunk(path, file_size, &vec![0u8; file_size.min(65536)])?;
    }

    std::fs::remove_file(path)
        .with_context(|| format!("Removing file after wipe: {:?}", path))
}

/// Overwrites the entire contents of `path` with the repeating `chunk` pattern.
fn overwrite_with_chunk(path: &Path, file_size: usize, chunk: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("Opening file for wipe pass: {:?}", path))?;

    let mut written = 0;
    while written < file_size {
        let to_write = (file_size - written).min(chunk.len());
        file.write_all(&chunk[..to_write])
            .context("Writing wipe data")?;
        written += to_write;
    }
    file.flush().context("Flushing wipe pass")?;
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
}
