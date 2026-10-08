//! `PATH` lookup used by Secure Mode.
//!
//! Only absolute `PATH` entries are searched (an empty or `.` entry would let
//! a file in the current directory shadow a real tool). On Windows only
//! `.exe` / `.com` images are accepted: `.bat` / `.cmd` run through `cmd.exe`
//! and would turn an allowed name into a shell.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const IMAGE_EXTENSIONS: &[&str] = &["exe", "com"];

/// Find `name` (a bare program name) in the directories of `path_var`.
/// Returns the absolute path of the first regular, executable match.
pub fn resolve_in_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path_var)
        .filter(|dir| dir.is_absolute())
        .flat_map(|dir| candidates(&dir, name))
        .find(|candidate| is_executable_file(candidate))
}

#[cfg(windows)]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let has_image_ext = Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        });
    if has_image_ext {
        vec![dir.join(name)]
    } else {
        IMAGE_EXTENSIONS
            .iter()
            .map(|ext| dir.join(format!("{name}.{ext}")))
            .collect()
    }
}

#[cfg(not(windows))]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    vec![dir.join(name)]
}

#[cfg(windows)]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(not(windows))]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn make_tool(dir: &Path, stem: &str) -> PathBuf {
        let p = dir.join(format!("{stem}.exe"));
        std::fs::write(&p, b"MZ").unwrap();
        p
    }

    #[cfg(not(windows))]
    fn make_tool(dir: &Path, stem: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(stem);
        std::fs::write(&p, b"#!").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn finds_tool_in_absolute_path_entry() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path(), "mytool");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(resolve_in_path("mytool", &path_var), Some(tool));
    }

    #[test]
    fn relative_path_entries_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        make_tool(dir.path(), "mytool");
        let path_var = std::env::join_paths([".", "relative/bin"]).unwrap();
        assert_eq!(resolve_in_path("mytool", &path_var), None);
    }

    #[test]
    fn missing_tool_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(resolve_in_path("nothing-here", &path_var), None);
    }

    #[cfg(windows)]
    #[test]
    fn batch_files_are_not_resolved() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tool.bat"), b"@echo off").unwrap();
        std::fs::write(dir.path().join("tool.cmd"), b"@echo off").unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(resolve_in_path("tool", &path_var), None);
        assert_eq!(resolve_in_path("tool.bat", &path_var), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn non_executable_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plain"), b"x").unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(resolve_in_path("plain", &path_var), None);
    }
}
