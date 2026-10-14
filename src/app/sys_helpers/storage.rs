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
