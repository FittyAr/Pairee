//! Regression tests of the data-safety fixes (audit 2026-10, FS-01..FS-08,
//! FEAT-01..03).

use super::{job, run};
use crate::fs::file_id::file_identity;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::part::{Replaced, part_path};

fn overwrite() -> TransferOptions {
    TransferOptions {
        conflict_resolution: "overwrite".into(),
        ..TransferOptions::default()
    }
}

/// FS-01: both panels on one folder, F5 / F6 with "overwrite" used to
/// truncate (copy) or delete (move) the file.
#[tokio::test]
async fn a_file_copied_or_moved_onto_itself_is_refused_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, b"precious").unwrap();
    for operation in [TransferOperation::Copy, TransferOperation::Move] {
        let results = run(job(operation, &[&file], dir.path(), overwrite()))
            .await
            .unwrap();
        assert_eq!(results.failed_files.len(), 1, "{operation:?}");
        assert!(results.completed_files.is_empty());
        assert_eq!(std::fs::read(&file).unwrap(), b"precious");
    }
    // "Rename" makes a duplicate next to it instead.
    let rename = TransferOptions {
        conflict_resolution: "rename".into(),
        ..TransferOptions::default()
    };
    let results = run(job(TransferOperation::Copy, &[&file], dir.path(), rename))
        .await
        .unwrap();
    assert_eq!(results.completed_files.len(), 1);
    assert_ne!(results.completed_files[0].dst, file);
    assert_eq!(std::fs::read(&file).unwrap(), b"precious");
}

/// FS-05: copying a folder into its own subfolder is refused.
#[tokio::test]
async fn a_folder_copied_into_its_subfolder_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    std::fs::create_dir_all(a.join("sub")).unwrap();
    std::fs::write(a.join("f.txt"), b"x").unwrap();
    let results = run(job(
        TransferOperation::Copy,
        &[&a],
        &a.join("sub"),
        overwrite(),
    ))
    .await
    .unwrap();
    assert_eq!(results.failed_files.len(), 1);
    assert!(!a.join("sub").join("a").exists());
}

/// FS-02: a cancelled overwrite keeps the original and leaves no
/// temporary file behind.
#[tokio::test]
async fn a_cancelled_overwrite_keeps_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("big.bin");
    std::fs::write(&src, vec![7u8; 8 * 1024 * 1024]).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let dst = out.join("big.bin");
    std::fs::write(&dst, b"original").unwrap();
    let options = TransferOptions {
        limit_bandwidth_rate: Some(1024 * 1024),
        ..overwrite()
    };
    let job = job(TransferOperation::Copy, &[&src], &out, options);
    let cancel = job.is_cancelled.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    assert!(run(job).await.is_err());
    assert_eq!(std::fs::read(&dst).unwrap(), b"original");
    assert!(!part_path(&dst).exists());
}

/// FS-03: verification reports what it checked; without it nothing is
/// claimed as verified.
#[tokio::test]
async fn verified_reflects_a_real_check() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.bin");
    std::fs::write(&src, vec![3u8; 100_000]).unwrap();
    for verify in [false, true] {
        let out = dir.path().join(format!("out{verify}"));
        std::fs::create_dir(&out).unwrap();
        let options = TransferOptions {
            verify_after_copy: verify,
            ..TransferOptions::default()
        };
        let results = run(job(TransferOperation::Copy, &[&src], &out, options))
            .await
            .unwrap();
        let done = &results.completed_files[0];
        assert_eq!(done.verified, verify);
        assert_eq!(done.dst_hash.is_some(), verify);
        assert_eq!(done.src_hash, done.dst_hash);
    }
}

/// UI-03: copy-on-write (or its fallback) and sparse copies are exact and
/// verified.
#[tokio::test]
async fn copy_on_write_and_sparse_copies_are_exact() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    let mut data = vec![0u8; 256 * 1024];
    data[100_000] = 42;
    std::fs::write(&src, &data).unwrap();
    for (cow, sparse) in [(true, false), (false, true)] {
        let out = dir.path().join(format!("out-{cow}-{sparse}"));
        std::fs::create_dir(&out).unwrap();
        let options = TransferOptions {
            use_copy_on_write: cow,
            produce_sparse_files: sparse,
            verify_after_copy: true,
            ..TransferOptions::default()
        };
        let results = run(job(TransferOperation::Copy, &[&src], &out, options))
            .await
            .unwrap();
        assert!(results.completed_files[0].verified);
        assert_eq!(std::fs::read(out.join("data.bin")).unwrap(), data);
    }
}

/// FEAT-01: a move within one volume renames (the file keeps its identity).
#[tokio::test]
async fn a_move_on_one_volume_is_a_rename() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("tree");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("f.txt"), b"x").unwrap();
    let id = file_identity(&src.join("f.txt"), false).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let results = run(job(
        TransferOperation::Move,
        &[&src],
        &out,
        TransferOptions::default(),
    ))
    .await
    .unwrap();
    assert_eq!(results.completed_files.len(), 1, "one rename of the folder");
    assert!(!src.exists());
    assert_eq!(
        file_identity(&out.join("tree").join("f.txt"), false).unwrap(),
        id
    );
}

/// FS-07: a move whose source cannot be deleted is reported as failed.
#[tokio::test]
async fn a_move_that_cannot_delete_its_source_fails() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let src = src_dir.join("f.txt");
    std::fs::write(&src, b"data").unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let Some(_lock) = lock_against_deletion(&src) else {
        return;
    };
    // Verification forces copy + delete (no rename).
    let options = TransferOptions {
        verify_after_copy: true,
        max_retries: 0,
        ..TransferOptions::default()
    };
    let results = run(job(TransferOperation::Move, &[&src], &out, options))
        .await
        .unwrap();
    assert_eq!(results.failed_files.len(), 1);
    assert!(results.completed_files.is_empty());
    assert!(src.exists());
    assert_eq!(std::fs::read(out.join("f.txt")).unwrap(), b"data");
}

/// Keeps `path` from being deleted while the guard lives (`None` where the
/// test cannot arrange that, e.g. running as root).
#[cfg(windows)]
fn lock_against_deletion(path: &std::path::Path) -> Option<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    // Others may read, not delete.
    const FILE_SHARE_READ: u32 = 1;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .ok()
}

#[cfg(unix)]
fn lock_against_deletion(path: &std::path::Path) -> Option<ReadOnlyDir> {
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent()?.to_path_buf();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).ok()?;
    let guard = ReadOnlyDir(dir);
    // Root ignores the permission bits.
    let probe = guard.0.join(".probe");
    if std::fs::write(&probe, b"").is_ok() {
        let _ = std::fs::remove_file(&probe);
        return None;
    }
    Some(guard)
}

#[cfg(unix)]
struct ReadOnlyDir(std::path::PathBuf);

#[cfg(unix)]
impl Drop for ReadOnlyDir {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// FS-08: moving a tree of relative links keeps them relative (and working).
#[tokio::test]
async fn relative_links_survive_a_move() {
    let dir = tempfile::tempdir().unwrap();
    let tree = dir.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    std::fs::write(tree.join("data.txt"), b"x").unwrap();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink("data.txt", tree.join("link"));
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file("data.txt", tree.join("link"));
    if made.is_err() {
        return;
    }
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    // Verification forces the per-file path (no rename).
    let options = TransferOptions {
        verify_after_copy: true,
        ..TransferOptions::default()
    };
    run(job(TransferOperation::Move, &[&tree], &out, options))
        .await
        .unwrap();
    let link = out.join("tree").join("link");
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        std::path::Path::new("data.txt")
    );
    assert_eq!(std::fs::read(&link).unwrap(), b"x");
}

/// A move never follows links, even with "follow links" set: the link is
/// moved and the folder it points to keeps its files.
#[tokio::test]
async fn a_move_moves_links_not_what_they_point_to() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), b"keep").unwrap();
    let tree = dir.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    if !crate::fs::walk::tests::symlink_dir(&outside, &tree.join("link")) {
        return;
    }
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let options = TransferOptions {
        follow_symlinks: true,
        ..TransferOptions::default()
    };
    run(job(TransferOperation::Move, &[&tree], &out, options))
        .await
        .unwrap();
    assert!(outside.join("keep.txt").exists());
    let moved = out.join("tree").join("link");
    assert!(
        std::fs::symlink_metadata(&moved)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!tree.exists());
}

/// FEAT-03: an overwritten file goes to the trash when asked.
#[tokio::test]
async fn replaced_files_go_to_the_trash() {
    let dir = tempfile::tempdir().unwrap();
    if !crate::fs::journal::trash::trash_round_trips_in(dir.path()) {
        return;
    }
    let src = dir.path().join("a.txt");
    std::fs::write(&src, b"new").unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("a.txt"), b"old").unwrap();
    let options = TransferOptions {
        trash_replaced: true,
        ..overwrite()
    };
    let results = run(job(TransferOperation::Copy, &[&src], &out, options))
        .await
        .unwrap();
    assert_eq!(results.completed_files[0].replaced, Replaced::Trashed);
    assert_eq!(std::fs::read(out.join("a.txt")).unwrap(), b"new");
    let mut index = crate::fs::journal::TrashIndex::load().unwrap();
    std::fs::remove_file(out.join("a.txt")).unwrap();
    index.restore(&out.join("a.txt")).unwrap();
    assert_eq!(std::fs::read(out.join("a.txt")).unwrap(), b"old");
}

/// A delete with a filter mask (a sync with a mask mirrors only what it
/// selects) removes the matching files only, and keeps the folder of
/// what it left out without reporting it as failed.
#[tokio::test]
async fn a_filtered_delete_keeps_what_the_mask_leaves_out() {
    let dir = tempfile::tempdir().unwrap();
    let tree = dir.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    std::fs::write(tree.join("a.rs"), b"a").unwrap();
    std::fs::write(tree.join("b.md"), b"b").unwrap();
    let options = TransferOptions {
        filter_mask: Some("*.rs".into()),
        delete_to_recycle_bin: false,
        ..TransferOptions::default()
    };
    let results = run(job(
        TransferOperation::Delete,
        &[&tree],
        std::path::Path::new(""),
        options,
    ))
    .await
    .unwrap();
    assert!(!tree.join("a.rs").exists());
    assert_eq!(std::fs::read(tree.join("b.md")).unwrap(), b"b");
    assert!(
        results.failed_files.is_empty(),
        "{:?}",
        results.failed_files
    );
}

/// APP2-44: the conflict policy and the speed limit changed in the panel
/// reach a job whose worker already copied its options.
#[tokio::test]
async fn live_option_changes_reach_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(src.join("a.txt"), vec![7u8; 64 * 1024]).unwrap();
    std::fs::write(dst.join("a.txt"), b"old").unwrap();
    // Started as "skip" at one byte per second: it would never finish.
    let started_with = TransferOptions {
        conflict_resolution: "skip".into(),
        limit_bandwidth_rate: Some(1),
        ..TransferOptions::default()
    };
    let job = job(
        TransferOperation::Copy,
        &[&src.join("a.txt")],
        &dst,
        started_with.clone(),
    );
    let live = std::sync::Arc::clone(&job.live);
    let edited = TransferOptions {
        conflict_resolution: "overwrite".into(),
        limit_bandwidth_rate: None,
        ..started_with
    };
    live.publish(&edited);
    let results = tokio::time::timeout(std::time::Duration::from_secs(20), run(job))
        .await
        .expect("the speed limit was lifted")
        .unwrap();
    assert_eq!(results.completed_files.len(), 1, "{results:?}");
    assert_eq!(std::fs::read(dst.join("a.txt")).unwrap().len(), 64 * 1024);
}
