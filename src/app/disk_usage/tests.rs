use super::*;
use std::fs;

/// root/{big/{x(100), inner/{y(5)}}, small.txt(3)}
fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("big/inner")).unwrap();
    fs::write(root.join("big/x"), [0u8; 100]).unwrap();
    fs::write(root.join("big/inner/y"), [0u8; 5]).unwrap();
    fs::write(root.join("small.txt"), [0u8; 3]).unwrap();
    dir
}

fn opened(root: &Path) -> DiskUsageState {
    let mut du = DiskUsageState::default();
    du.open(root.to_path_buf(), Default::default());
    assert!(!du.is_scanning(), "no runtime: the scan runs inline");
    du
}

#[test]
fn scan_lists_largest_first() {
    let dir = tree();
    let du = opened(dir.path());
    assert_eq!(du.tree().unwrap().size.bytes, 108);
    let names: Vec<_> = du
        .current()
        .unwrap()
        .children
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["big", "small.txt"]);
    assert_eq!(du.selected_path(), Some(dir.path().join("big")));
}

#[test]
fn enter_and_leave_restore_cursor() {
    let dir = tree();
    let mut du = opened(dir.path());
    assert!(du.enter());
    assert_eq!(du.current_path(), dir.path().join("big"));
    du.cursor = 1;
    assert!(du.enter());
    assert_eq!(du.current_path(), dir.path().join("big").join("inner"));
    assert!(!du.enter(), "a file cannot be opened");
    assert!(du.leave());
    assert_eq!(du.cursor, 1);
    assert!(du.leave());
    assert_eq!(du.cursor, 0);
    assert!(!du.leave(), "stops at the scanned root");
}

#[test]
fn deleted_items_leave_the_tree() {
    let dir = tree();
    let mut du = opened(dir.path());
    du.cursor = 1;
    let small = du.selected_path().unwrap();
    let elsewhere = dir.path().parent().unwrap().join("unrelated");
    assert!(
        !du.remove_deleted([elsewhere.as_path()]),
        "outside the scan"
    );
    assert!(du.remove_deleted([small.as_path()]));
    assert_eq!(du.tree().unwrap().size.bytes, 105);
    assert_eq!(du.current().unwrap().children.len(), 1);
    assert_eq!(du.cursor, 0);
}

#[test]
fn finished_delete_jobs_prune_the_tree_without_a_rescan() {
    use crate::fs::transfer::job::{TransferJob, TransferOperation, TransferResults};
    let dir = tree();
    let mut du = opened(dir.path());
    let big = dir.path().join("big");
    let mut results = TransferResults::default();
    let start = std::time::Instant::now();
    let done = crate::fs::transfer::control::done(&big, PathBuf::new(), 0, start);
    results.completed_files.push(done);
    let job = |op| TransferJob::new(op, vec![big.clone()], PathBuf::new(), Default::default());
    assert!(!du.job_finished(&job(TransferOperation::Copy), &results));
    // The folder is still on disk: the tree trusts the job, not a poll.
    assert!(du.job_finished(&job(TransferOperation::Delete), &results));
    assert_eq!(du.tree().unwrap().size.bytes, 3);
    assert_eq!(du.selected_path(), Some(dir.path().join("small.txt")));
}

#[test]
fn reopen_same_folder_keeps_cache_and_rescan_refreshes() {
    let dir = tree();
    let mut du = opened(dir.path());
    du.enter();
    fs::write(dir.path().join("big/new"), [0u8; 1000]).unwrap();
    du.open(dir.path().to_path_buf(), Default::default());
    assert_eq!(du.tree().unwrap().size.bytes, 108, "cached");
    assert_eq!(du.current_path(), dir.path().join("big"));
    du.rescan();
    assert_eq!(du.tree().unwrap().size.bytes, 1108);
    assert_eq!(du.current_path(), dir.path().join("big"), "view kept");
}

#[test]
fn rescan_after_folder_vanished_falls_back_to_parent() {
    let dir = tree();
    let mut du = opened(dir.path());
    du.enter();
    du.enter();
    fs::remove_dir_all(dir.path().join("big/inner")).unwrap();
    du.rescan();
    assert_eq!(du.current_path(), dir.path().join("big"));
}
