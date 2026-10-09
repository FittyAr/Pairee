use super::options::TransferOptions;
use std::fs::Metadata;
use std::path::Path;

/// Preserva marcas de tiempo (creación, modificación, acceso) y atributos de archivo
/// (permisos Unix o atributos de archivo Windows) del origen en el destino.
pub fn preserve_metadata(src: &Path, dst: &Path, options: &TransferOptions) -> std::io::Result<()> {
    let src_meta = std::fs::symlink_metadata(src)?;

    // 1. Preservar Timestamps si está configurado
    if options.preserve_timestamps {
        preserve_times(&src_meta, dst);
    }

    // 2. Preservar Permisos y Atributos de archivo
    if options.preserve_attributes {
        preserve_attributes(src, &src_meta, dst);
    }

    // 3. Preservar ACLs (Security descriptor)
    if options.preserve_acl {
        preserve_acl(src, dst);
    }

    Ok(())
}

/// Intentar establecer atime y mtime de forma cross-platform.
fn preserve_times(src_meta: &Metadata, dst: &Path) {
    let atime = filetime::FileTime::from_last_access_time(src_meta);
    let mtime = filetime::FileTime::from_last_modification_time(src_meta);
    let _ = filetime::set_file_times(dst, atime, mtime);
}

/// En Unix, transferir los permisos ordinarios.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn preserve_attributes(_src: &Path, src_meta: &Metadata, dst: &Path) {
    let _ = std::fs::set_permissions(dst, src_meta.permissions());
}

/// En Windows, transferir los atributos de archivo y los Alternate Data Streams.
#[cfg(target_os = "windows")]
fn preserve_attributes(src: &Path, src_meta: &Metadata, dst: &Path) {
    use std::os::windows::fs::MetadataExt;

    let wide_path = wide(dst);
    // Llamar a SetFileAttributesW de windows-sys
    unsafe {
        windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(
            wide_path.as_ptr(),
            src_meta.file_attributes(),
        );
    }
    copy_alternate_streams(src, dst);
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn preserve_attributes(_src: &Path, _src_meta: &Metadata, _dst: &Path) {}

/// Copiar Alternate Data Streams (ADS) si NTFS.
#[cfg(target_os = "windows")]
fn copy_alternate_streams(src: &Path, dst: &Path) {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
        WIN32_FIND_STREAM_DATA,
    };

    let src_wide = wide(src);
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
                .unwrap_or(296);
            let stream_name = String::from_utf16_lossy(&find_data.cStreamName[..name_len]);

            if !stream_name.is_empty()
                && stream_name != "::$DATA"
                && let Some(clean_name) = stream_name.strip_suffix(":$DATA")
            {
                let src_ads = format!("{}{}", src.to_string_lossy(), clean_name);
                let dst_ads = format!("{}{}", dst.to_string_lossy(), clean_name);
                let _ = std::fs::copy(src_ads, dst_ads);
            }

            if FindNextStreamW(handle, &mut find_data as *mut _ as *mut _) == 0 {
                break;
            }
        }
        FindClose(handle);
    }
}

/// Copia propietario, grupo y DACL del origen al destino.
#[cfg(target_os = "windows")]
fn preserve_acl(src: &Path, dst: &Path) {
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

#[cfg(not(target_os = "windows"))]
fn preserve_acl(_src: &Path, _dst: &Path) {}

/// NUL-terminated UTF-16 path for Win32 calls.
#[cfg(target_os = "windows")]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
