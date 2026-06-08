use anyhow::{Context, Result};
use std::path::Path;

/// Creates a symbolic link at `dest` pointing to `src`.
///
/// On UNIX/Linux: uses `std::os::unix::fs::symlink`.
/// On Windows: uses `std::os::windows::fs::symlink_file` or `symlink_dir`.
pub fn create_symlink(src: &Path, dest: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest).with_context(|| {
            format!("Creating symlink {:?} → {:?}", dest, src)
        })
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(src, dest).with_context(|| {
                format!("Creating dir symlink {:?} → {:?}", dest, src)
            })
        } else {
            std::os::windows::fs::symlink_file(src, dest).with_context(|| {
                format!("Creating file symlink {:?} → {:?}", dest, src)
            })
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        anyhow::bail!("Symlinks are not supported on this platform")
    }
}

/// Creates a hard link at `dest` pointing to the same inode as `src`.
///
/// Hard links only work for files (not directories) and within the same filesystem.
pub fn create_hardlink(src: &Path, dest: &Path) -> Result<()> {
    std::fs::hard_link(src, dest)
        .with_context(|| format!("Creating hard link {:?} → {:?}", dest, src))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_create_symlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("original.txt");
        std::fs::write(&src, b"hello").unwrap();
        let dest = dir.path().join("link.txt");

        create_symlink(&src, &dest).expect("symlink should succeed");
        assert!(dest.exists());
        assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
    }

    #[test]
    fn test_create_hardlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("original.txt");
        std::fs::write(&src, b"hello hardlink").unwrap();
        let dest = dir.path().join("hardlink.txt");

        create_hardlink(&src, &dest).expect("hardlink should succeed");
        assert!(dest.exists());
        // Both files should have the same content
        assert_eq!(
            std::fs::read(&src).unwrap(),
            std::fs::read(&dest).unwrap()
        );
    }
}
