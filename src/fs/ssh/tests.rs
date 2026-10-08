//! Unit tests for the server-independent parts of the SSH backend:
//! listing entry mapping, `..` handling, known_hosts path and verdicts.

use super::connection::{host_key_verdict, known_hosts_path_in};
use super::sftp_ops::{is_real_child, sftp_entry};
use crate::fs::vfs::parent_entry;
use ssh2::{CheckResult, FileStat};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;

fn stat(mode: u32, size: Option<u64>, mtime: Option<u64>) -> FileStat {
    FileStat {
        size,
        uid: None,
        gid: None,
        perm: Some(mode | 0o644),
        atime: None,
        mtime,
    }
}

#[test]
fn parent_entry_points_to_parent_for_nested_dirs() {
    let entry = parent_entry(Path::new("/home/user/docs"), false).unwrap();
    assert_eq!(entry.name, "..");
    assert_eq!(entry.path, PathBuf::from("/home/user"));
    assert!(entry.is_dir);
    assert!(!entry.is_symlink);
    assert_eq!(entry.size, 0);
    assert!(entry.modified.is_none());
}

#[test]
fn parent_entry_of_top_level_dir_is_root() {
    let entry = parent_entry(Path::new("/srv"), false).unwrap();
    assert_eq!(entry.path, PathBuf::from("/"));
}

#[test]
fn parent_entry_at_root_depends_on_setting() {
    for root in ["/", ""] {
        assert!(parent_entry(Path::new(root), false).is_none(), "{root:?}");
        let entry = parent_entry(Path::new(root), true).unwrap();
        assert_eq!(entry.name, "..");
        assert_eq!(entry.path, PathBuf::from(root));
    }
}

#[test]
fn real_children_exclude_dot_entries() {
    assert!(is_real_child("file.txt"));
    assert!(is_real_child(".hidden"));
    assert!(is_real_child("..."));
    for skip in ["", ".", ".."] {
        assert!(!is_real_child(skip), "{skip:?}");
    }
}

#[test]
fn sftp_entry_maps_regular_file_metadata() {
    let path = PathBuf::from("/data/report.csv");
    let entry = sftp_entry(path.clone(), &stat(S_IFREG, Some(42), Some(1_000))).unwrap();
    assert_eq!(entry.name, "report.csv");
    assert_eq!(entry.path, path);
    assert_eq!(entry.size, 42);
    assert!(!entry.is_dir);
    assert!(!entry.is_symlink);
    assert_eq!(
        entry.modified,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000))
    );
}

#[test]
fn sftp_entry_maps_directories_and_symlinks() {
    let dir = sftp_entry(PathBuf::from("/data/sub"), &stat(S_IFDIR, None, None)).unwrap();
    assert!(dir.is_dir);
    assert!(!dir.is_symlink);
    assert_eq!(dir.size, 0, "missing size defaults to zero");
    assert!(dir.modified.is_none());

    let link = sftp_entry(PathBuf::from("/data/ln"), &stat(S_IFLNK, Some(7), None)).unwrap();
    assert!(link.is_symlink);
    assert!(!link.is_dir);
}

#[test]
fn sftp_entry_flags_dot_files_as_hidden() {
    let st = stat(S_IFREG, Some(1), None);
    let hidden = sftp_entry(PathBuf::from("/data/.env"), &st).unwrap();
    assert!(hidden.hidden);
    assert_eq!(hidden.name, ".env");
    assert!(!sftp_entry(PathBuf::from("/data/a"), &st).unwrap().hidden);
}

#[test]
fn sftp_entry_skips_dot_entries() {
    let st = stat(S_IFDIR, None, None);
    // `Path` yields no file name for `..` or the root; both must be dropped.
    assert!(sftp_entry(PathBuf::from("/data/.."), &st).is_none());
    assert!(sftp_entry(PathBuf::from("/"), &st).is_none());
}

#[test]
fn known_hosts_path_is_under_home_ssh_dir() {
    let home = PathBuf::from("home").join("alice");
    let path = known_hosts_path_in(Some(OsString::from(home.as_os_str()))).unwrap();
    assert_eq!(path, home.join(".ssh").join("known_hosts"));
}

#[test]
fn known_hosts_path_requires_a_home_dir() {
    assert!(known_hosts_path_in(None).is_none());
    assert!(known_hosts_path_in(Some(OsString::new())).is_none());
}

#[test]
fn only_matching_host_keys_are_accepted() {
    let kh = Path::new("known_hosts");
    assert!(host_key_verdict(CheckResult::Match, "example.org", 22, Some(kh)).is_ok());

    let mismatch = host_key_verdict(CheckResult::Mismatch, "example.org", 22, Some(kh))
        .unwrap_err()
        .to_string();
    assert!(mismatch.contains("example.org:22"), "{mismatch}");
    assert!(mismatch.contains("man-in-the-middle"), "{mismatch}");

    let unknown = host_key_verdict(CheckResult::NotFound, "example.org", 2222, Some(kh))
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("example.org:2222"), "{unknown}");
    assert!(unknown.contains("known_hosts"), "{unknown}");

    let failure = host_key_verdict(CheckResult::Failure, "example.org", 22, None)
        .unwrap_err()
        .to_string();
    assert!(failure.contains("check failed"), "{failure}");
}
