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
    let meta = std::fs::metadata(path).with_context(|| format!("Reading metadata: {:?}", path))?;

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
    let (mode, owner, nlinks) = { (0u32, "N/A".to_string(), 1u64) };

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
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    bits.iter()
        .map(|(mask, ch)| if mode & mask != 0 { *ch } else { '-' })
        .collect()
}

#[cfg(unix)]
fn get_unix_owner_name(uid: u32) -> String {
    // The previous implementation re-parsed `/etc/passwd` on every call,
    // which turned a directory listing of an `ls -l`-style view into
    // O(N×M) where N is the number of files and M the number of users
    // in /etc/passwd. On a multi-user box with thousands of system
    // accounts that is a real cost. We now memoise the result per-uid
    // for the lifetime of the process.
    //
    // We deliberately do NOT call `getpwuid(3)` from libc: it can block
    // indefinitely on NSS backends (LDAP, sssd, nscd) and we are
    // already in a synchronous rendering path. The file read of
    // /etc/passwd is the local-only fast path; if the user is missing
    // from it (typical for LDAP/SSSD users) we fall back to the
    // numeric UID, which is what the old code did too.
    use std::collections::HashMap;
    use std::sync::OnceLock;

    static CACHE: OnceLock<std::sync::Mutex<HashMap<u32, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock() {
        if let Some(name) = map.get(&uid) {
            return name.clone();
        }
    }

    let resolved = lookup_owner_in_passwd(uid).unwrap_or_else(|| uid.to_string());
    if let Ok(mut map) = cache.lock() {
        map.insert(uid, resolved.clone());
    }
    resolved
}

#[cfg(unix)]
fn lookup_owner_in_passwd(uid: u32) -> Option<String> {
    let content = std::fs::read_to_string("/etc/passwd").ok()?;
    let target = uid.to_string();
    for line in content.lines() {
        let mut parts = line.split(':');
        let name = parts.next()?;
        let _pass = parts.next()?;
        let uid_str = parts.next()?;
        if uid_str.trim() == target {
            return Some(name.to_string());
        }
    }
    None
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
