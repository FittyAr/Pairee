use super::endpoint::TransferEndpoint;
use super::options::TransferOptions;
use std::path::Path;

/// Preserve the metadata of `src` onto `dst` according to `options`.
///
/// `src_endpoint` is used to *read* metadata from the source; `dst_endpoint`
/// is used to *write* it on the destination. For the local-only case
/// (which the rest of the engine still does until Phase 4) both endpoints
/// are `TransferEndpoint::Local`. Once the worker carries endpoints
/// per-panel, this function transparently handles cross-endpoint
/// preservation (Local → Ssh, Ssh → Ssh, etc.).
///
/// Capabilities that an endpoint cannot satisfy are silently skipped
/// with a `log::warn!` so a single non-supporting feature does not
/// fail the whole transfer. Examples:
///
/// * xattr/ACL/ADS on SSH (SFTP v3 has no portable primitive for them
///   and we opted out of `session.exec` shell-out).
/// * Windows-only ACL/attribute APIs on Unix endpoints (and vice versa).
///
/// Returning `Ok(())` is the contract even when individual fields are
/// skipped; the per-file `FileTransferResult` already records whether
/// metadata was fully preserved via the engine's event log.
pub fn preserve_metadata(
    src_endpoint: &TransferEndpoint,
    src: &Path,
    dst_endpoint: &TransferEndpoint,
    dst: &Path,
    options: &TransferOptions,
) -> Result<(), super::endpoint::EndpointError> {
    // Read source metadata once; everything below derives from this.
    let src_meta = src_endpoint.lstat(src)?;

    // ------------------------------------------------------------------
    // 1. Timestamps (cross-platform; works for Local and Ssh).
    // ------------------------------------------------------------------
    if options.preserve_timestamps {
        if let (Some(atime), Some(mtime)) = (src_meta.accessed, src_meta.modified) {
            if let Err(e) = dst_endpoint.set_timestamps(dst, atime, mtime) {
                log::warn!(
                    "preserve_metadata: failed to set timestamps on {}: {}",
                    dst.display(),
                    e
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // 2. Unix mode / Windows file attribute.
    //    Endpoint::set_permissions handles both worlds; for Ssh it
    //    uses fsetstat with the `perm` field.
    // ------------------------------------------------------------------
    if options.preserve_attributes {
        if let Some(mode) = src_meta.mode {
            if let Err(e) = dst_endpoint.set_permissions(dst, mode) {
                log::warn!(
                    "preserve_metadata: failed to set permissions on {}: {}",
                    dst.display(),
                    e
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // 3. Windows ACL / Alternate Data Streams.
    //    These are Windows-only APIs that are not abstracted by
    //    TransferEndpoint (the engine does not surface them over SSH
    //    either). They run only when the destination is Local on
    //    Windows, and only when the source is also Local on Windows
    //    (so we can read the security descriptor with the Windows
    //    API).
    // ------------------------------------------------------------------
    #[cfg(windows)]
    {
        if options.preserve_acl && src_endpoint.is_local() && dst_endpoint.is_local() {
            preserve_windows_acl(src, dst);
        }
        if options.preserve_streams && src_endpoint.is_local() && dst_endpoint.is_local() {
            copy_ads(src, dst);
        }
    }

    // ------------------------------------------------------------------
    // 4. Unix xattr (Local only — SSH without shell-out cannot).
    // ------------------------------------------------------------------
    #[cfg(unix)]
    {
        if src_endpoint.supports_xattr() && dst_endpoint.supports_xattr() {
            if let Ok(names) = src_endpoint.list_xattrs(src) {
                for name in names {
                    match src_endpoint.get_xattr(src, &name) {
                        Ok(Some(value)) => {
                            if let Err(e) = dst_endpoint.set_xattr(dst, &name, &value) {
                                log::warn!(
                                    "preserve_metadata: failed to set xattr {} on {}: {}",
                                    name,
                                    dst.display(),
                                    e
                                );
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!(
                                "preserve_metadata: failed to read xattr {} on {}: {}",
                                name,
                                src.display(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    let _ = src_meta; // keep variable alive if all branches are cfg'd out
    Ok(())
}

// =========================================================================
//   Windows-specific helpers
// =========================================================================

#[cfg(windows)]
fn preserve_windows_acl(src: &Path, dst: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::SetFileSecurityW;
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR,
    };

    let mut src_wide: Vec<u16> = src.as_os_str().encode_wide().collect();
    src_wide.push(0);
    let mut dst_wide: Vec<u16> = dst.as_os_str().encode_wide().collect();
    dst_wide.push(0);

    let security_info =
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

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

#[cfg(windows)]
fn copy_ads(src: &Path, dst: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
        WIN32_FIND_STREAM_DATA,
    };

    let mut src_wide: Vec<u16> = src.as_os_str().encode_wide().collect();
    src_wide.push(0);

    let mut find_data: WIN32_FIND_STREAM_DATA = unsafe { std::mem::zeroed() };
    let handle = unsafe {
        FindFirstStreamW(
            src_wide.as_ptr(),
            FindStreamInfoStandard,
            &mut find_data as *mut _ as *mut _,
            0,
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return;
    }

    loop {
        let name_len = find_data
            .cStreamName
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(296);
        let stream_name = String::from_utf16_lossy(&find_data.cStreamName[..name_len]);

        if !stream_name.is_empty() && stream_name != "::$DATA" {
            if let Some(clean_name) = stream_name.strip_suffix(":$DATA") {
                let src_ads = format!("{}{}", src.to_string_lossy(), clean_name);
                let dst_ads = format!("{}{}", dst.to_string_lossy(), clean_name);
                let _ = std::fs::copy(src_ads, dst_ads);
            }
        }

        if unsafe { FindNextStreamW(handle, &mut find_data as *mut _ as *mut _) } == 0 {
            break;
        }
    }

    unsafe {
        FindClose(handle);
    }
}

// =========================================================================
//   Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn ep() -> TransferEndpoint {
        TransferEndpoint::Local
    }

    #[test]
    fn preserve_timestamps_local() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a");
        let dst = tmp.path().join("b");
        std::fs::write(&src, b"x").unwrap();
        std::fs::write(&dst, b"x").unwrap();

        // Set known timestamps on the source.
        let atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_500);
        let src_ep = ep();
        src_ep.set_timestamps(&src, atime, mtime).unwrap();

        // Make sure destination is *different* so the test is
        // meaningful.
        let other_atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
        let other_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_500);
        src_ep
            .set_timestamps(&dst, other_atime, other_mtime)
            .unwrap();

        let mut opts = TransferOptions::default();
        opts.preserve_timestamps = true;
        opts.preserve_attributes = false;
        opts.preserve_acl = false;

        preserve_metadata(&ep(), &src, &ep(), &dst, &opts).unwrap();

        let meta = std::fs::metadata(&dst).unwrap();
        let got_mtime = meta.modified().unwrap();
        let diff = got_mtime
            .duration_since(mtime)
            .unwrap_or_else(|e| e.duration());
        assert!(
            diff < Duration::from_secs(2),
            "mtime should match source after preserve_metadata (got drift {:?})",
            diff
        );
    }

    #[test]
    fn preserve_attributes_local_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tmp = tempfile::tempdir().unwrap();
            let src = tmp.path().join("a");
            let dst = tmp.path().join("b");
            std::fs::write(&src, b"x").unwrap();
            std::fs::write(&dst, b"x").unwrap();
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644)).unwrap();

            let mut opts = TransferOptions::default();
            opts.preserve_timestamps = false;
            opts.preserve_attributes = true;
            opts.preserve_acl = false;

            preserve_metadata(&ep(), &src, &ep(), &dst, &opts).unwrap();

            let meta = std::fs::metadata(&dst).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn preserve_xattr_local_unix() {
        #[cfg(unix)]
        {
            let tmp = tempfile::tempdir().unwrap();
            let src = tmp.path().join("a");
            let dst = tmp.path().join("b");
            std::fs::write(&src, b"x").unwrap();
            std::fs::write(&dst, b"x").unwrap();

            // Some tmpfs / CI filesystems don't support xattr; skip
            // silently if set fails.
            let src_ep = ep();
            if src_ep.set_xattr(&src, "user.pairee_test", b"v").is_err() {
                return;
            }

            let mut opts = TransferOptions::default();
            opts.preserve_timestamps = false;
            opts.preserve_attributes = false;
            opts.preserve_acl = false;

            preserve_metadata(&ep(), &src, &ep(), &dst, &opts).unwrap();

            let got = src_ep.get_xattr(&dst, "user.pairee_test").unwrap();
            assert_eq!(got.as_deref(), Some(&b"v"[..]));
        }
    }

    #[test]
    fn no_preserve_flags_is_a_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a");
        let dst = tmp.path().join("b");
        std::fs::write(&src, b"x").unwrap();
        std::fs::write(&dst, b"y").unwrap();

        let opts = TransferOptions {
            preserve_timestamps: false,
            preserve_attributes: false,
            preserve_acl: false,
            preserve_streams: false,
            ..TransferOptions::default()
        };

        // Should not error even though everything is disabled.
        preserve_metadata(&ep(), &src, &ep(), &dst, &opts).unwrap();

        // The destination content must be untouched.
        assert_eq!(std::fs::read(&dst).unwrap(), b"y");
    }
}
