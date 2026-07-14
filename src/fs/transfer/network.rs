use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

/// Determina si un path se encuentra en una unidad de red local (LAN).
pub fn is_lan_path(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        // En Windows, resolver la raíz del volumen
        let path_str = path.to_string_lossy();

        // Comprobar si es una ruta UNC directa (ej: \\server\share)
        if path_str.starts_with(r"\\") {
            return true;
        }

        // Obtener la raíz del volumen (ej: C:\)
        let root = if let Some(disk) = path_str.get(0..3) {
            if disk.chars().nth(1) == Some(':') && disk.chars().nth(2) == Some('\\') {
                disk.to_string()
            } else {
                return false;
            }
        } else {
            return false;
        };

        let root_wide: Vec<u16> = std::ffi::OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            // GetDriveTypeW de windows-sys. DRIVE_REMOTE es 4.
            let drive_type =
                windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root_wide.as_ptr());
            drive_type == 4 // DRIVE_REMOTE
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // En Linux, leer /proc/mounts
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let mount_point = parts[1];
                    let fs_type = parts[2];

                    // Comprobar si el path empieza con el mount point
                    if path.starts_with(mount_point) {
                        // Tipos comunes de FS de red
                        if fs_type == "nfs"
                            || fs_type == "cifs"
                            || fs_type == "smbfs"
                            || fs_type == "nfs4"
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Obtiene el espacio libre disponible en bytes en la unidad que contiene el path destino.
pub fn get_free_space(path: &Path) -> std::io::Result<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;

        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        let mut free_bytes_available = 0u64;
        let mut total_number_of_bytes = 0u64;
        let mut total_number_of_free_bytes = 0u64;

        unsafe {
            let res = windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_number_of_bytes,
                &mut total_number_of_free_bytes,
            );
            if res == 0 {
                if let Some(parent) = path.parent() {
                    return get_free_space(parent);
                }
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(free_bytes_available)
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = std::process::Command::new("df")
            .arg("--output=avail")
            .arg("-k")
            .arg(path)
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = text.lines().nth(1) {
                if let Ok(kb) = line.trim().parse::<u64>() {
                    return Ok(kb * 1024);
                }
            }
        }
        Ok(u64::MAX)
    }
}
