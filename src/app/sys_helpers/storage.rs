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
        // The previous list was a hard-coded set of well-known mount
        // points. It was missing the user's actual $HOME (which on
        // Fedora Silverblue / RHEL / OpenSUSE Tumbleweed lives under
        // `/var/home`, and on plain distros can be `/home/<user>`),
        // and also missing bind mounts of arbitrary user-chosen
        // paths. We now:
        //   1. Always include the root and the canonical "external
        //      media" anchors.
        //   2. Include $HOME if it is set and is *not* the same as
        //      `/` (we don't want a duplicate entry for the root
        //      user on a server).
        //   3. Probe the well-known alternative home locations so the
        //      user can jump to them from the drive bar even if the
        //      default path doesn't apply.
        let mut candidates: Vec<String> = vec![
            "/".to_string(),
            "/home".to_string(),
            "/media".to_string(),
            "/mnt".to_string(),
            "/tmp".to_string(),
            // Distros that do not put HOME under /home
            "/var/home".to_string(),
            "/usr/home".to_string(),
        ];
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() && home != "/" {
                candidates.push(home);
            }
        }
        for p in candidates {
            if std::path::Path::new(&p).exists() {
                // Avoid duplicates (e.g. /home is already in the list
                // and could also be $HOME).
                if !drives.iter().any(|d| d == &p) {
                    drives.push(p);
                }
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
