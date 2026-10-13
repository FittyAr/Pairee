//! "Use copy-on-write": the destination shares the source's data blocks
//! until either is changed, so the copy is instant and takes no space.
//!
//! * Linux: the `FICLONE` ioctl (Btrfs, XFS, bcachefs, OCFS2...).
//! * macOS: `clonefile` (APFS).
//! * Windows: on ReFS volumes (Dev Drives included) the copy goes through
//!   `CopyFileExW`, which clones blocks where Windows supports it (Windows 11
//!   24H2 and later) and copies them otherwise.
//!
//! Everywhere else, or when the filesystem refuses, [`clone_file`] fails and
//! the caller falls back to the normal copy.

use std::io;
use std::path::Path;

/// Makes `dst` (which must not exist) a copy-on-write clone of `src`.
pub fn clone_file(src: &Path, dst: &Path) -> io::Result<()> {
    imp::clone_file(src, dst)
}

#[cfg(target_os = "linux")]
mod imp {
    use std::os::fd::AsRawFd;
    use std::path::Path;

    pub fn clone_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        let source = std::fs::File::open(src)?;
        let target = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dst)?;
        // SAFETY: both descriptors are open for the duration of the call.
        let res = unsafe { libc::ioctl(target.as_raw_fd(), libc::FICLONE, source.as_raw_fd()) };
        if res == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        drop(target);
        let _ = std::fs::remove_file(dst);
        Err(err)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn clone_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        let to_c = |p: &Path| {
            CString::new(p.as_os_str().as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        };
        let (src, dst) = (to_c(src)?, to_c(dst)?);
        // SAFETY: both strings are valid NUL-terminated paths.
        if unsafe { libc::clonefile(src.as_ptr(), dst.as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationByHandleW;

    /// File system name of the volume holding the open `file`.
    fn volume_fs_name(file: &std::fs::File) -> Option<String> {
        let mut name = [0u16; 64];
        // SAFETY: the buffers are valid for the given lengths; unused
        // outputs are null.
        let ok = unsafe {
            GetVolumeInformationByHandleW(
                file.as_raw_handle() as _,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                name.as_mut_ptr(),
                name.len() as u32,
            )
        };
        if ok == 0 {
            return None;
        }
        let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Some(String::from_utf16_lossy(&name[..len]))
    }

    pub fn clone_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        let source = std::fs::File::open(src)?;
        if volume_fs_name(&source).as_deref() != Some("ReFS") {
            return Err(std::io::ErrorKind::Unsupported.into());
        }
        drop(source);
        // `CopyFileExW` block-clones on ReFS where Windows supports it.
        std::fs::copy(src, dst).map(|_| ())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    use std::path::Path;

    pub fn clone_file(_src: &Path, _dst: &Path) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where cloning works the copy is exact; elsewhere it fails cleanly
    /// and leaves nothing behind.
    #[test]
    fn clone_is_exact_or_fails_without_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        std::fs::write(&src, b"copy on write").unwrap();
        match clone_file(&src, &dst) {
            Ok(()) => assert_eq!(std::fs::read(&dst).unwrap(), b"copy on write"),
            Err(_) => assert!(!dst.exists()),
        }
    }
}
