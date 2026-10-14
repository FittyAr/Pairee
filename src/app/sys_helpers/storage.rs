/// Returns the system drives/mounts for the current OS.
///
/// Windows asks the system which drive letters exist (`GetLogicalDrives`)
/// instead of probing each root: probing a mapped network drive whose
/// server is offline blocks until the network gives up, and this runs on
/// the UI thread. Such a drive is listed; reading it happens in the
/// background like any panel listing.
pub fn get_system_drives() -> Vec<String> {
    let mut drives = Vec::new();
    #[cfg(windows)]
    {
        // SAFETY: no arguments; returns a bitmask of the drive letters.
        let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
        drives.extend(
            (0..26u8)
                .filter(|bit| mask & (1 << bit) != 0)
                .map(|bit| format!("{}:\\", (b'A' + bit) as char)),
        );
    }
    #[cfg(not(windows))]
    {
        for p in ["/", "/home", "/media", "/mnt", "/tmp"] {
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_drive_of_the_current_folder_is_listed() {
        let cwd = std::env::current_dir().unwrap();
        let root = cwd
            .ancestors()
            .last()
            .unwrap()
            .to_string_lossy()
            .to_uppercase();
        let drives = super::get_system_drives();
        assert!(
            drives.iter().any(|d| root.starts_with(&d.to_uppercase())
                || d.to_uppercase().starts_with(&root)),
            "{root} in {drives:?}"
        );
    }
}
