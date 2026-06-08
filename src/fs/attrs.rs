use anyhow::{Context, Result};
use std::path::Path;
use std::time::SystemTime;

/// File attribute snapshot (cross-platform subset).
#[derive(Debug, Clone)]
pub struct FileAttrs {
    pub path: std::path::PathBuf,
    /// UNIX permission mode bits (rwxrwxrwx), 0 on Windows.
    pub mode: u32,
    /// Whether the file is read-only.
    pub readonly: bool,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time.
    pub modified: Option<SystemTime>,
    /// Creation time (available on Windows and some UNIX variants).
    pub created: Option<SystemTime>,
    /// Owner name (UNIX) or "N/A" on Windows.
    pub owner: String,
    /// Number of hard links to this inode.
    pub nlinks: u64,
}

/// Reads the file attributes for the given path.
pub fn read_attrs(path: &Path) -> Result<FileAttrs> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("Reading metadata: {:?}", path))?;

    let readonly = meta.permissions().readonly();
    let size = meta.len();
    let modified = meta.modified().ok();
    let created = meta.created().ok();

    #[cfg(unix)]
    let (mode, owner, nlinks) = {
        use std::os::unix::fs::MetadataExt;
        let uid = meta.uid();
        let owner_name = get_unix_owner_name(uid);
        (meta.mode(), owner_name, meta.nlink())
    };

    #[cfg(not(unix))]
    let (mode, owner, nlinks) = {
        (0u32, "N/A".to_string(), 1u64)
    };

    Ok(FileAttrs {
        path: path.to_path_buf(),
        mode,
        readonly,
        size,
        modified,
        created,
        owner,
        nlinks,
    })
}

// Expose set_readonly utility function for metadata changes.
/// Sets the read-only flag on the file.
pub fn set_readonly(path: &Path, readonly: bool) -> Result<()> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("Reading metadata for chmod: {:?}", path))?;
    let mut perms = meta.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("Setting permissions on {:?}", path))
}

// This utility function is prepared for the interactive chmod attributes dialog.
/// Sets UNIX permission mode bits on the file (no-op on Windows).
pub fn set_unix_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("Setting UNIX mode {:o} on {:?}", mode, path))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(()) // No-op on non-UNIX
    }
}

/// Formats a UNIX mode u32 as a human-readable string, e.g. "rwxr-xr--".
pub fn format_unix_mode(mode: u32) -> String {
    let bits = [
        (0o400, 'r'), (0o200, 'w'), (0o100, 'x'),
        (0o040, 'r'), (0o020, 'w'), (0o010, 'x'),
        (0o004, 'r'), (0o002, 'w'), (0o001, 'x'),
    ];
    bits.iter()
        .map(|(mask, ch)| if mode & mask != 0 { *ch } else { '-' })
        .collect()
}

#[cfg(unix)]
fn get_unix_owner_name(uid: u32) -> String {
    // Use getpwuid if available via libc; fallback to numeric UID.
    // We avoid pulling in libc directly — read /etc/passwd instead.
    if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let mut parts = line.split(':');
            if let (Some(name), _, Some(uid_str)) =
                (parts.next(), parts.next(), parts.next())
            {
                if uid_str.trim() == uid.to_string() {
                    return name.to_string();
                }
            }
        }
    }
    uid.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_unix_mode() {
        assert_eq!(format_unix_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_unix_mode(0o644), "rw-r--r--");
        assert_eq!(format_unix_mode(0o000), "---------");
    }

    #[test]
    fn test_read_attrs_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello attrs").unwrap();

        let attrs = read_attrs(&path).expect("read_attrs should succeed");
        assert_eq!(attrs.size, 11);
        assert!(!attrs.readonly);
    }

    #[test]
    fn test_set_readonly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ro.txt");
        std::fs::write(&path, b"content").unwrap();

        set_readonly(&path, true).expect("set readonly");
        let attrs = read_attrs(&path).unwrap();
        assert!(attrs.readonly);

        // Restore for cleanup
        set_readonly(&path, false).unwrap();
    }
}
