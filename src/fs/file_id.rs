//! Identity of a filesystem object, independent of the path used to reach
//! it: `(device, inode)` on Unix, `(volume serial, file index)` on Windows.
//! Two paths with the same identity are the same file (hard links, links,
//! different spellings or letter case).

use std::io;
use std::path::{Path, PathBuf};

/// What identifies one file or folder on its filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileIdentity {
    /// Device (or volume) and inode (or file index).
    Id(u64, u64),
    /// A canonical path, for filesystems without inodes (SFTP).
    Path(PathBuf),
}

/// Identity of the object at `path`; with `follow` a final link is
/// resolved, otherwise the link itself is identified.
pub fn file_identity(path: &Path, follow: bool) -> io::Result<FileIdentity> {
    imp::file_identity(path, follow)
}

#[cfg(unix)]
mod imp {
    use super::FileIdentity;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    pub fn file_identity(path: &Path, follow: bool) -> std::io::Result<FileIdentity> {
        let meta = if follow {
            std::fs::metadata(path)?
        } else {
            std::fs::symlink_metadata(path)?
        };
        Ok(FileIdentity::Id(meta.dev(), meta.ino()))
    }
}

#[cfg(windows)]
mod imp {
    use super::FileIdentity;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        GetFileInformationByHandle,
    };

    pub fn file_identity(path: &Path, follow: bool) -> std::io::Result<FileIdentity> {
        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
        if !follow {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }
        // No access rights: only the attributes are queried, so files
        // locked by other programs can still be identified.
        let file = std::fs::OpenOptions::new()
            .access_mode(0)
            .custom_flags(flags)
            .open(crate::fs::transfer::direct_io::to_long_path(path))?;
        // SAFETY: `info` is plain data the call fills in; the handle is
        // valid for the life of `file`.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
        Ok(FileIdentity::Id(
            u64::from(info.dwVolumeSerialNumber),
            index,
        ))
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use super::FileIdentity;
    use std::path::Path;

    pub fn file_identity(path: &Path, _follow: bool) -> std::io::Result<FileIdentity> {
        Ok(FileIdentity::Path(std::fs::canonicalize(path)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same_file(a: &Path, b: &Path) -> bool {
        match (file_identity(a, true), file_identity(b, true)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }

    #[test]
    fn same_file_sees_through_spellings_and_hard_links() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        std::fs::write(&a, b"x").unwrap();
        let spelled = dir.path().join(".").join("a.txt");
        assert!(same_file(&a, &spelled));
        let hard = dir.path().join("hard.txt");
        std::fs::hard_link(&a, &hard).unwrap();
        assert!(same_file(&a, &hard));
        let other = dir.path().join("b.txt");
        std::fs::write(&other, b"x").unwrap();
        assert!(!same_file(&a, &other));
        assert!(!same_file(&a, &dir.path().join("missing")));
    }

    #[cfg(windows)]
    #[test]
    fn same_file_ignores_letter_case_on_windows() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("Name.txt");
        std::fs::write(&a, b"x").unwrap();
        assert!(same_file(&a, &dir.path().join("NAME.TXT")));
    }
}
