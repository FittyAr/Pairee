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

/// Owner name of a file with metadata `meta` (its uid when the name is
/// unknown) on Unix; `None` where the platform does not tell it cheaply.
pub fn owner_of(meta: &std::fs::Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = meta.uid();
        Some(super::identity::user_name_of(uid).unwrap_or_else(|| uid.to_string()))
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        None
    }
}

/// Reads the file attributes for the given path.
pub fn read_attrs(path: &Path) -> Result<FileAttrs> {
    let meta = std::fs::metadata(path).with_context(|| format!("Reading metadata: {:?}", path))?;

    let readonly = meta.permissions().readonly();
    let size = meta.len();
    let modified = meta.modified().ok();
    let created = meta.created().ok();

    let owner = owner_of(&meta).unwrap_or_else(|| "N/A".to_string());
    #[cfg(unix)]
    let (mode, nlinks) = {
        use std::os::unix::fs::MetadataExt;
        (meta.mode(), meta.nlink())
    };

    #[cfg(not(unix))]
    let (mode, nlinks) = (0u32, 1u64);

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

/// What the Attributes dialog applies: the typed octal mode (if any) and
/// the read-only flag, which wins over the mode's write bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttrChange {
    pub mode: Option<u32>,
    pub readonly: bool,
}

/// Permission bits of a mode (no file type bits).
const PERMISSION_BITS: u32 = 0o7777;
const WRITE_BITS: u32 = 0o222;
const OWNER_WRITE: u32 = 0o200;

impl AttrChange {
    /// The permission bits to set on an entry whose mode is `current`, for
    /// sources that only have POSIX modes (SFTP): read-only clears every
    /// write bit, writable restores the owner's when none is left.
    pub fn resulting_mode(self, current: u32) -> u32 {
        let mode = self.mode.unwrap_or(current) & PERMISSION_BITS;
        if self.readonly {
            mode & !WRITE_BITS
        } else if mode & WRITE_BITS == 0 {
            mode | OWNER_WRITE
        } else {
            mode
        }
    }

    /// Applies the change to a local entry (mode first, then the flag).
    pub fn apply_local(self, path: &Path) -> std::io::Result<()> {
        let failed = |key: &str, e: anyhow::Error| {
            std::io::Error::other(crate::config::localization::t(key).replace("{}", &e.to_string()))
        };
        if let Some(mode) = self.mode {
            set_unix_mode(path, mode).map_err(|e| failed("error_set_unix_mode_failed", e))?;
        }
        set_readonly(path, self.readonly).map_err(|e| failed("error_set_readonly_failed", e))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_flag_wins_over_the_mode() {
        let change = |mode, readonly| AttrChange { mode, readonly };
        assert_eq!(change(Some(0o755), true).resulting_mode(0), 0o555);
        assert_eq!(change(Some(0o444), false).resulting_mode(0), 0o644);
        assert_eq!(change(Some(0o640), false).resulting_mode(0), 0o640);
        assert_eq!(change(None, false).resulting_mode(0o100_644), 0o644);
    }

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
