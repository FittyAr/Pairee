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
        // En Linux, leer /proc/mounts y elegir el match más largo
        // (específico) entre los mounts con filesystem de red.
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            let mut best_len: usize = 0;
            let mut best_match = false;
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let mount_point = parts[1];
                let fs_type = parts[2];

                // Filtro rápido por tipo de FS
                let is_net = matches!(fs_type, "nfs" | "nfs4" | "cifs" | "smbfs");
                if !is_net {
                    continue;
                }

                // Verificación de path boundary: el path debe empezar con
                // el mount point seguido de un separador, o ser igual al
                // mount point. Sin esto, un mount en "/mnt" matchearía
                // falsamente con el path "/mntfoo/...".
                if path_under_mount(path, mount_point) {
                    if mount_point.len() > best_len {
                        best_len = mount_point.len();
                        best_match = true;
                    }
                }
            }
            return best_match;
        }
        false
    }
}

/// Returns true if `path` is exactly `mount_point` or a path strictly below it.
/// Wrapper over `Path::starts_with` that documents the component-wise
/// semantics; the function lives next to the call site so the contract is
/// obvious to anyone touching this code.
#[cfg(not(target_os = "windows"))]
fn path_under_mount(path: &Path, mount_point: &str) -> bool {
    path.starts_with(mount_point)
}
