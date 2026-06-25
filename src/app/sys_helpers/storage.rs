use std::path::Path;

/// Returns the system drives/mounts for the current OS.
pub fn get_system_drives() -> Vec<String> {
    let mut drives = Vec::new();
    if cfg!(target_os = "windows") {
        for drive_letter in b'A'..=b'Z' {
            let path = format!("{}:\\", drive_letter as char);
            if std::path::Path::new(&path).exists() {
                drives.push(path);
            }
        }
    } else {
        let paths = vec!["/", "/home", "/media", "/mnt", "/tmp"];
        for p in paths {
            if std::path::Path::new(p).exists() {
                drives.push(p.to_string());
            }
        }
    }
    if drives.is_empty() {
        drives.push("/".to_string());
    }
    drives
}

/// Returns available free space in bytes for the volume containing `path`.
/// Uses native Win32 `GetDiskFreeSpaceExW` on Windows; reads /proc/mounts on other platforms.
/// Returns `None` if the query fails.
pub fn get_free_space(path: &Path) -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_bytes: u64 = 0;
        let mut _total_bytes: u64 = 0;
        let mut _total_free: u64 = 0;
        // SAFETY: We pass valid non-null pointers for output parameters.
        let ret = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes,
                &mut _total_bytes,
                &mut _total_free,
            )
        };
        if ret != 0 {
            return Some(free_bytes);
        }
        return None;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Use `df` command as a portable cross-platform fallback
        let output = std::process::Command::new("df")
            .arg("--output=avail")
            .arg("-k")
            .arg(path)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let kb: u64 = text.lines().nth(1)?.trim().parse().ok()?;
        Some(kb * 1024)
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available_to_caller: *mut u64,
        lp_total_number_of_bytes: *mut u64,
        lp_total_number_of_free_bytes: *mut u64,
    ) -> i32;
}
