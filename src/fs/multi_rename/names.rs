//! File-name rules of the filesystem the files live on: which names are
//! valid and whether two names refer to the same entry.

use std::path::Path;

/// Naming rules of the target filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetFs {
    /// Windows rules: `<>:"/\|?*`, control characters, trailing dot/space
    /// and reserved device names (`CON`, `NUL`, `COM1`...) are invalid.
    pub windows_names: bool,
    /// `a.txt` and `A.TXT` are the same entry.
    pub case_insensitive: bool,
}

impl TargetFs {
    /// Rules of the local machine.
    pub const fn local() -> Self {
        Self {
            windows_names: cfg!(windows),
            case_insensitive: cfg!(any(windows, target_os = "macos")),
        }
    }

    /// Rules of an SFTP server (POSIX: case-sensitive, only `/` and NUL
    /// forbidden).
    pub const fn remote() -> Self {
        Self {
            windows_names: false,
            case_insensitive: false,
        }
    }

    /// Comparison key: two names with the same key are the same entry.
    pub fn key(self, name: &str) -> String {
        if self.case_insensitive {
            name.to_lowercase()
        } else {
            name.to_string()
        }
    }

    /// [`Self::key`] of a whole path.
    pub fn path_key(self, path: &Path) -> String {
        self.key(&path.to_string_lossy())
    }

    /// `true` when `name` can be used as a file name.
    pub fn is_valid_name(self, name: &str) -> bool {
        if name.is_empty() || name == "." || name == ".." {
            return false;
        }
        if name.chars().any(|c| c == '/' || c == '\0') {
            return false;
        }
        !self.windows_names || is_valid_windows_name(name)
    }
}

const WINDOWS_FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const WINDOWS_RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL"];

fn is_valid_windows_name(name: &str) -> bool {
    if name
        .chars()
        .any(|c| c.is_control() || WINDOWS_FORBIDDEN.contains(&c))
    {
        return false;
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return false;
    }
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    !is_reserved_device(stem)
}

/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9` (any case).
fn is_reserved_device(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&upper.as_str()) {
        return true;
    }
    let numbered = upper
        .strip_prefix("COM")
        .or_else(|| upper.strip_prefix("LPT"));
    matches!(numbered.map(str::as_bytes), Some([b'1'..=b'9']))
}
