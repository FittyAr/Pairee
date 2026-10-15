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
        const NETWORK_FS: [&str; 5] = ["nfs", "nfs4", "cifs", "smbfs", "smb3"];
        mount_fs_type(path).is_some_and(|fs| NETWORK_FS.contains(&fs.as_str()))
    }
}

/// File system type of the mount holding `path` (the longest matching mount
/// point in `/proc/mounts`); `None` where that file does not exist.
#[cfg(not(target_os = "windows"))]
pub fn mount_fs_type(path: &Path) -> Option<String> {
    mount_of(path).map(|(_, fs_type)| fs_type)
}

/// The device and file system type of the mount `path` lies on (the
/// longest mount point that contains it, from `/proc/mounts`).
#[cfg(not(target_os = "windows"))]
pub fn mount_of(path: &Path) -> Option<(String, String)> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    mount_in(&mounts, path)
}

/// [`mount_of`] over the text of a mounts table.
#[cfg(not(target_os = "windows"))]
fn mount_in(mounts: &str, path: &Path) -> Option<(String, String)> {
    mounts
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (device, mount_point, fs_type) = (parts.next()?, parts.next()?, parts.next()?);
            let mount_point = unescape_mount_field(mount_point);
            path.starts_with(&mount_point).then(|| {
                let found = (device.to_string(), fs_type.to_string());
                (mount_point.len(), found)
            })
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, found)| found)
}

/// A field of `/proc/mounts` with its octal escapes decoded: the kernel
/// writes a space as `\040` (and tab, newline and backslash alike), so a
/// mount point such as `/mnt/My Disk` would otherwise never match.
#[cfg(not(target_os = "windows"))]
fn unescape_mount_field(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|d| d.iter().all(|b| (b'0'..=b'7').contains(b)));
        match (bytes[i], octal) {
            (b'\\', Some(digits)) => {
                let value = digits.iter().fold(0u32, |n, d| n * 8 + u32::from(d - b'0'));
                out.push(u8::try_from(value).unwrap_or(b'?'));
                i += 4;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Free bytes available to the user on the volume holding `path` (the
/// panel status line and the transfer low-space check). An error means
/// unknown.
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
        use std::os::unix::ffi::OsStrExt;
        // `statvfs` on every Unix (the `df --output` flag is GNU-only).
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `c_path` is NUL-terminated and `stats` is a valid out value.
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        // The field widths differ per platform.
        #[allow(clippy::unnecessary_cast)]
        let (blocks, size) = (stats.f_bavail as u64, stats.f_frsize as u64);
        Ok(blocks.saturating_mul(size))
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn mount_points_with_spaces_are_matched() {
        let mounts = "/dev/sda1 / ext4 rw 0 0\n//srv/share /mnt/My\\040Disk cifs rw 0 0\n";
        let found = mount_in(mounts, Path::new("/mnt/My Disk/docs/a.txt"));
        assert_eq!(found, Some(("//srv/share".into(), "cifs".into())));
        let other = mount_in(mounts, Path::new("/mnt/other/a.txt"));
        assert_eq!(other, Some(("/dev/sda1".into(), "ext4".into())));
    }

    #[test]
    fn octal_escapes_are_decoded() {
        assert_eq!(unescape_mount_field("a\\040b\\011c\\134d"), "a b\tc\\d");
        assert_eq!(unescape_mount_field("plain\\9x"), "plain\\9x");
    }

    #[test]
    fn free_space_of_the_current_folder_is_known() {
        assert!(get_free_space(Path::new(".")).is_ok());
    }
}
