//! Copying metadata from a source to its copy: times, access mode
//! (permission bits / ACL), attributes (Windows attributes; extended
//! attributes on Unix, see [`super::xattrs`]) and alternate data streams,
//! each controlled by its own option.
//!
//! A copied link gets only its own times (`lutimes`): permissions,
//! attributes and streams of a link would be applied to what it points to.

use super::options::TransferOptions;
use std::fs::Metadata;
use std::path::Path;

/// Applies the metadata `options` ask for from `src` to `dst`.
pub fn preserve_metadata(src: &Path, dst: &Path, options: &TransferOptions) -> std::io::Result<()> {
    let src_meta = std::fs::symlink_metadata(src)?;
    if src_meta.file_type().is_symlink() {
        if options.preserve_timestamps {
            let (atime, mtime) = times(&src_meta);
            filetime::set_symlink_file_times(dst, atime, mtime)?;
        }
        return Ok(());
    }
    // Streams first: the attributes may make the file read-only.
    if options.preserve_streams {
        platform::copy_alternate_streams(src, dst);
    }
    if options.preserve_permissions {
        platform::copy_access_mode(src, &src_meta, dst);
    }
    if options.preserve_acl {
        platform::copy_acl(src, dst);
    }
    if options.preserve_attributes {
        platform::copy_attributes(src, &src_meta, dst);
    }
    // Times last: writing streams or attributes may touch them.
    if options.preserve_timestamps {
        let (atime, mtime) = times(&src_meta);
        filetime::set_file_times(dst, atime, mtime)?;
    }
    Ok(())
}

fn times(meta: &Metadata) -> (filetime::FileTime, filetime::FileTime) {
    (
        filetime::FileTime::from_last_access_time(meta),
        filetime::FileTime::from_last_modification_time(meta),
    )
}

#[cfg(unix)]
mod platform {
    use std::fs::Metadata;
    use std::path::Path;

    /// Unix permission bits.
    pub fn copy_access_mode(_src: &Path, src_meta: &Metadata, dst: &Path) {
        let _ = std::fs::set_permissions(dst, src_meta.permissions());
    }

    /// POSIX ACLs (Linux).
    pub fn copy_acl(src: &Path, dst: &Path) {
        crate::fs::transfer::xattrs::copy_posix_acl(src, dst);
    }

    /// Extended attributes.
    pub fn copy_attributes(src: &Path, _src_meta: &Metadata, dst: &Path) {
        crate::fs::transfer::xattrs::copy_extended_attributes(src, dst);
    }

    pub fn copy_alternate_streams(_src: &Path, _dst: &Path) {}
}

#[cfg(windows)]
mod platform {
    use crate::fs::transfer::direct_io::to_long_path;
    use std::fs::Metadata;
    use std::path::Path;

    /// NUL-terminated UTF-16 path for Win32 calls (with the `\\?\` prefix,
    /// so long paths work).
    fn wide(path: &Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        to_long_path(path)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// The Windows access mode is the access control list.
    pub fn copy_access_mode(src: &Path, _src_meta: &Metadata, dst: &Path) {
        copy_acl(src, dst);
    }

    /// Read-only, hidden, system and archive attributes.
    pub fn copy_attributes(_src: &Path, src_meta: &Metadata, dst: &Path) {
        use std::os::windows::fs::MetadataExt;
        let path = wide(dst);
        // SAFETY: `path` is a valid NUL-terminated wide string.
        unsafe {
            windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(
                path.as_ptr(),
                src_meta.file_attributes(),
            );
        }
    }

    /// Copies the named NTFS streams (`file:name`).
    pub fn copy_alternate_streams(src: &Path, dst: &Path) {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{
            FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
            WIN32_FIND_STREAM_DATA,
        };

        let src_wide = wide(src);
        // SAFETY: `find_data` is plain data filled in by the calls; the
        // handle is closed before returning.
        unsafe {
            let mut find_data: WIN32_FIND_STREAM_DATA = std::mem::zeroed();
            let handle = FindFirstStreamW(
                src_wide.as_ptr(),
                FindStreamInfoStandard,
                &mut find_data as *mut _ as *mut _,
                0,
            );
            if handle == INVALID_HANDLE_VALUE {
                return;
            }
            loop {
                let name_len = find_data
                    .cStreamName
                    .iter()
                    .position(|&x| x == 0)
                    .unwrap_or(find_data.cStreamName.len());
                let stream_name = String::from_utf16_lossy(&find_data.cStreamName[..name_len]);
                if stream_name != "::$DATA"
                    && let Some(clean_name) = stream_name.strip_suffix(":$DATA")
                {
                    let mut src_ads = to_long_path(src).into_os_string();
                    src_ads.push(clean_name);
                    let mut dst_ads = to_long_path(dst).into_os_string();
                    dst_ads.push(clean_name);
                    let _ = std::fs::copy(src_ads, dst_ads);
                }
                if FindNextStreamW(handle, &mut find_data as *mut _ as *mut _) == 0 {
                    break;
                }
            }
            FindClose(handle);
        }
    }

    /// Owner, group and DACL.
    pub fn copy_acl(src: &Path, dst: &Path) {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
        use windows_sys::Win32::Security::SetFileSecurityW;
        use windows_sys::Win32::Security::{
            DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR,
        };

        let src_wide = wide(src);
        let dst_wide = wide(dst);
        let security_info =
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
        let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: out-pointers are valid; the descriptor is freed with
        // `LocalFree` as the API requires.
        unsafe {
            let res = GetNamedSecurityInfoW(
                src_wide.as_ptr(),
                SE_FILE_OBJECT,
                security_info,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut security_descriptor,
            );
            if res == 0 && !security_descriptor.is_null() {
                let _ = SetFileSecurityW(dst_wide.as_ptr(), security_info, security_descriptor);
                LocalFree(security_descriptor as _);
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::fs::Metadata;
    use std::path::Path;

    pub fn copy_access_mode(_src: &Path, _src_meta: &Metadata, _dst: &Path) {}
    pub fn copy_acl(_src: &Path, _dst: &Path) {}
    pub fn copy_attributes(_src: &Path, _src_meta: &Metadata, _dst: &Path) {}
    pub fn copy_alternate_streams(_src: &Path, _dst: &Path) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copied link gets its own times; the file it points to keeps its
    /// permissions and times.
    #[test]
    fn link_metadata_never_reaches_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        std::fs::write(&target, b"x").unwrap();
        let old = filetime::FileTime::from_unix_time(1_000_000_000, 0);
        filetime::set_file_times(&target, old, old).unwrap();
        let before = std::fs::metadata(&target).unwrap();

        let src_link = dir.path().join("src_link");
        let dst_link = dir.path().join("dst_link");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &src_link)
            .and_then(|()| std::os::unix::fs::symlink(&target, &dst_link));
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&target, &src_link)
            .and_then(|()| std::os::windows::fs::symlink_file(&target, &dst_link));
        if made.is_err() {
            return;
        }
        let options = TransferOptions {
            preserve_attributes: true,
            preserve_permissions: true,
            ..TransferOptions::default()
        };
        preserve_metadata(&src_link, &dst_link, &options).unwrap();
        let after = std::fs::metadata(&target).unwrap();
        assert_eq!(before.permissions(), after.permissions());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    }
}
