use std::path::Path;
use super::options::TransferOptions;

/// Preserva marcas de tiempo (creación, modificación, acceso) y atributos de archivo
/// (permisos Unix o atributos de archivo Windows) del origen en el destino.
pub fn preserve_metadata(src: &Path, dst: &Path, options: &TransferOptions) -> std::io::Result<()> {
    let src_meta = std::fs::symlink_metadata(src)?;

    // 1. Preservar Timestamps si está configurado
    if options.preserve_timestamps {
        let atime = filetime::FileTime::from_last_access_time(&src_meta);
        let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
        
        // Intentar establecer atime y mtime de forma cross-platform
        let _ = filetime::set_file_times(dst, atime, mtime);
    }

    // 2. Preservar Permisos y Atributos de archivo
    if options.preserve_attributes {
        // En Unix, transferir los permisos ordinarios
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let perms = src_meta.permissions();
            let _ = std::fs::set_permissions(dst, perms);
        }

        // En Windows, transferir los atributos de archivo usando la API de Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::os::windows::fs::MetadataExt;
            
            let attrs = src_meta.file_attributes();
            let mut wide_path: Vec<u16> = dst.as_os_str().encode_wide().collect();
            wide_path.push(0);

            // Llamar a SetFileAttributesW de windows-sys
            unsafe {
                windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(
                    wide_path.as_ptr(),
                    attrs,
                );
            }

            // Copiar Alternate Data Streams (ADS) si NTFS y está activo
            unsafe {
                use windows_sys::Win32::Storage::FileSystem::{
                    FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard, WIN32_FIND_STREAM_DATA, FindClose,
                };
                use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

                let mut src_wide: Vec<u16> = src.as_os_str().encode_wide().collect();
                src_wide.push(0);

                let mut find_data: WIN32_FIND_STREAM_DATA = std::mem::zeroed();
                let handle = FindFirstStreamW(
                    src_wide.as_ptr(),
                    FindStreamInfoStandard,
                    &mut find_data as *mut _ as *mut _,
                    0,
                );

                if handle != INVALID_HANDLE_VALUE {
                    loop {
                        let name_len = find_data.cStreamName.iter().position(|&x| x == 0).unwrap_or(296);
                        let stream_name = String::from_utf16_lossy(&find_data.cStreamName[..name_len]);

                        if !stream_name.is_empty() && stream_name != "::$DATA" {
                            if let Some(clean_name) = stream_name.strip_suffix(":$DATA") {
                                let src_ads = format!("{}{}", src.to_string_lossy(), clean_name);
                                let dst_ads = format!("{}{}", dst.to_string_lossy(), clean_name);
                                let _ = std::fs::copy(src_ads, dst_ads);
                            }
                        }

                        if FindNextStreamW(handle, &mut find_data as *mut _ as *mut _) == 0 {
                            break;
                        }
                    }
                    FindClose(handle);
                }
            }
        }
    }

    // 3. Preservar ACLs (Security descriptor)
    if options.preserve_acl {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
            use windows_sys::Win32::Security::{
                OWNER_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, DACL_SECURITY_INFORMATION,
                PSECURITY_DESCRIPTOR
            };
            use windows_sys::Win32::Security::SetFileSecurityW;
            use windows_sys::Win32::Foundation::LocalFree;

            let mut src_wide: Vec<u16> = src.as_os_str().encode_wide().collect();
            src_wide.push(0);
            let mut dst_wide: Vec<u16> = dst.as_os_str().encode_wide().collect();
            dst_wide.push(0);

            let security_info = OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
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
                    let _ = SetFileSecurityW(
                        dst_wide.as_ptr(),
                        security_info,
                        security_descriptor,
                    );
                    LocalFree(security_descriptor as _);
                }
            }
        }
    }

    Ok(())
}
