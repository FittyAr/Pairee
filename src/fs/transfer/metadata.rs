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
        }
    }

    Ok(())
}
