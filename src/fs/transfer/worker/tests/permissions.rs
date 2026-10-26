//! Deleting what the user has no permission to delete (Unix): every refusal
//! is reported once, with its path and reason, marked as denied (so the
//! app can offer to retry with administrator rights), and nothing else is
//! touched. Root ignores these permissions, so the tests skip as root.

use super::{job, run};
use crate::fs::transfer::job::{TransferOperation, TransferResults};
use crate::fs::transfer::options::TransferOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

/// Puts the permissions of a folder back on drop, so the temporary folder
/// can be removed even when an assertion fails.
struct Restore(PathBuf);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// `folder` with a file in it, inside a new temporary folder.
fn tree(folder: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(folder);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("f.txt"), b"x").unwrap();
    (dir, path)
}

async fn delete(path: &Path, to_trash: bool) -> TransferResults {
    let options = TransferOptions {
        delete_to_recycle_bin: to_trash,
        ..TransferOptions::default()
    };
    run(job(
        TransferOperation::Delete,
        &[path],
        Path::new(""),
        options,
    ))
    .await
    .unwrap()
}

/// The failures as `(path, denied)`.
fn failures(results: &TransferResults) -> Vec<(PathBuf, bool)> {
    results
        .failed_files
        .iter()
        .map(|f| (f.src.clone(), f.denied))
        .collect()
}

#[tokio::test]
async fn a_folder_in_a_read_only_folder_is_reported_as_denied() {
    if crate::fs::is_elevated() {
        return;
    }
    let (_dir, parent) = tree("parent");
    let victim = parent.join("victim");
    std::fs::create_dir(&victim).unwrap();
    std::fs::write(victim.join("kept.txt"), b"x").unwrap();
    let before = mode(&victim);
    chmod(&parent, 0o555);
    let _restore = Restore(parent.clone());

    let results = delete(&victim, false).await;
    assert_eq!(failures(&results), [(victim.clone(), true)]);
    assert!(
        results.failed_files[0].error.contains("ermission denied"),
        "{}",
        results.failed_files[0].error
    );
    // Refused before anything inside it went: the folder stays whole.
    assert!(victim.join("kept.txt").exists());
    assert_eq!(mode(&victim), before);
}

#[tokio::test]
async fn an_unreadable_subfolder_is_reported_once_with_its_reason() {
    if crate::fs::is_elevated() {
        return;
    }
    let (_dir, top) = tree("top");
    let locked = top.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("inside.txt"), b"x").unwrap();
    chmod(&locked, 0o000);
    let _restore = Restore(locked.clone());

    let results = delete(&top, false).await;
    // No "Directory not empty" for `locked` or `top`: the unreadable
    // folder is the reason, reported once.
    assert_eq!(failures(&results), [(locked.clone(), true)]);
    assert!(!top.join("f.txt").exists());
    assert_eq!(mode(&locked), 0);
}

#[tokio::test]
async fn files_of_a_read_only_subfolder_are_denied_and_keep_their_permissions() {
    if crate::fs::is_elevated() {
        return;
    }
    let (_dir, top) = tree("top");
    let read_only = top.join("ro");
    std::fs::create_dir(&read_only).unwrap();
    let kept = read_only.join("kept.txt");
    std::fs::write(&kept, b"x").unwrap();
    chmod(&kept, 0o444);
    chmod(&read_only, 0o555);
    let _restore = Restore(read_only.clone());

    let results = delete(&top, false).await;
    assert_eq!(failures(&results), [(kept.clone(), true)]);
    assert!(kept.exists());
    // The failed attempt does not leave the file writable.
    assert_eq!(mode(&kept), 0o444);
    assert_eq!(mode(&read_only), 0o555);
}

#[tokio::test]
async fn trashing_from_a_read_only_folder_says_why_and_keeps_the_item() {
    if crate::fs::is_elevated() {
        return;
    }
    let (_dir, parent) = tree("parent");
    let victim = parent.join("victim");
    std::fs::create_dir(&victim).unwrap();
    std::fs::write(victim.join("f.txt"), b"x").unwrap();
    chmod(&parent, 0o555);
    let _restore = Restore(parent.clone());

    let results = delete(&victim, true).await;
    assert_eq!(failures(&results), [(victim.clone(), true)]);
    let error = &results.failed_files[0].error;
    assert!(error.contains(&*victim.to_string_lossy()), "{error}");
    assert!(error.contains("ermission denied"), "{error}");
    // A readable reason, not the trash library's debug dump.
    assert!(!error.contains("FileSystem {"), "{error}");
    assert!(victim.join("f.txt").exists());
}
