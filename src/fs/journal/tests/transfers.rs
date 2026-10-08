use super::super::check::SkipReason;
use super::super::{FileBatch, check};
use super::{FsCommand, p, read, run_job, run_recorded, runnable_inverse, write};
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;
use std::path::PathBuf;

fn options() -> TransferOptions {
    TransferOptions {
        conflict_resolution: "overwrite".into(),
        ..TransferOptions::default()
    }
}

/// The job the undo executor submits for a copy/move batch.
fn pairs_job(operation: TransferOperation, batch: &FileBatch) -> TransferJob {
    let pairs = batch
        .files
        .iter()
        .map(|f| (f.from.clone(), f.to.clone()))
        .collect();
    let options = TransferOptions {
        conflict_resolution: "skip".into(),
        ..TransferOptions::default()
    };
    TransferJob::for_pairs(operation, pairs, options).with_prune_dirs(batch.prune_dirs.clone())
}

fn batch(command: &FsCommand) -> &FileBatch {
    match command {
        FsCommand::Move(b) | FsCommand::Copy(b) | FsCommand::RemoveCopies(b) => b,
        other => panic!("not a batch: {other:?}"),
    }
}

#[tokio::test]
async fn move_of_a_folder_tree_across_dirs_is_undone_and_redone() {
    let root = tempfile::tempdir().unwrap();
    let (src, dst) = (p(root.path(), "src"), p(root.path(), "dst"));
    write(&src.join("tree").join("a.txt"), "a");
    write(&src.join("tree").join("sub").join("b.txt"), "b");
    std::fs::create_dir_all(&dst).unwrap();
    let job = TransferJob::new(
        TransferOperation::Move,
        vec![src.join("tree")],
        dst.clone(),
        options(),
    );
    let entry = run_recorded(job, None).await.unwrap();
    assert!(!src.join("tree").exists());
    assert_eq!(
        batch(&entry).created_dirs.len(),
        2,
        "dst/tree and dst/tree/sub"
    );

    let undo = runnable_inverse(&entry);
    let undone = run_recorded(
        pairs_job(TransferOperation::Move, batch(&undo)),
        Some(&undo),
    )
    .await
    .unwrap();
    assert_eq!(read(&src.join("tree").join("sub").join("b.txt")), "b");
    assert!(!dst.join("tree").exists(), "created folders are pruned");

    let redo = runnable_inverse(&undone);
    run_job(pairs_job(TransferOperation::Move, batch(&redo))).await;
    assert_eq!(read(&dst.join("tree").join("a.txt")), "a");
    assert!(!src.join("tree").exists());
}

#[tokio::test]
async fn move_undo_skips_files_changed_since() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (p(root.path(), "a.txt"), p(root.path(), "b.txt"));
    write(&a, "a");
    write(&b, "b");
    let out = p(root.path(), "out");
    std::fs::create_dir_all(&out).unwrap();
    let job = TransferJob::new(
        TransferOperation::Move,
        vec![a.clone(), b.clone()],
        out.clone(),
        options(),
    );
    let entry = run_recorded(job, None).await.unwrap();
    write(&out.join("b.txt"), "edited after the move");

    let checked = check(&entry.inverse().unwrap());
    assert_eq!(checked.skipped.len(), 1);
    assert_eq!(checked.skipped[0].path, out.join("b.txt"));
    assert_eq!(checked.skipped[0].reason, SkipReason::Changed);
    let undo = checked.runnable.unwrap();
    run_job(pairs_job(TransferOperation::Move, batch(&undo))).await;
    assert_eq!(read(&a), "a");
    assert!(!b.exists(), "the edited file stays where it is");
}

#[tokio::test]
async fn copy_undo_deletes_only_new_copies() {
    let root = tempfile::tempdir().unwrap();
    let (src, dst) = (p(root.path(), "src"), p(root.path(), "dst"));
    write(&src.join("new.txt"), "new");
    write(&src.join("old.txt"), "source");
    write(&dst.join("old.txt"), "precious");
    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![src.join("new.txt"), src.join("old.txt")],
        dst.clone(),
        options(),
    );
    let entry = run_recorded(job, None).await.unwrap();
    assert_eq!(read(&dst.join("old.txt")), "source");

    let checked = check(&entry.inverse().unwrap());
    assert_eq!(checked.skipped.len(), 1);
    assert_eq!(checked.skipped[0].reason, SkipReason::Replaced);
    let undo = checked.runnable.unwrap();
    let copies: Vec<PathBuf> = batch(&undo).files.iter().map(|f| f.to.clone()).collect();
    assert_eq!(copies, vec![dst.join("new.txt")]);
    let delete = TransferJob::new(
        TransferOperation::Delete,
        copies,
        PathBuf::new(),
        TransferOptions::default(),
    );
    let undone = run_recorded(delete, Some(&undo)).await.unwrap();
    assert!(!dst.join("new.txt").exists());
    assert!(dst.join("old.txt").exists(), "overwritten file is kept");
    assert!(src.join("new.txt").exists());

    // Redo copies the file again.
    let redo = runnable_inverse(&undone);
    run_job(pairs_job(TransferOperation::Copy, batch(&redo))).await;
    assert_eq!(read(&dst.join("new.txt")), "new");
}

#[tokio::test]
async fn copy_undo_keeps_a_copy_whose_original_is_gone() {
    let root = tempfile::tempdir().unwrap();
    let (src, dst) = (p(root.path(), "a.txt"), p(root.path(), "b.txt"));
    write(&src, "a");
    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![src.clone()],
        dst.clone(),
        options(),
    );
    let entry = run_recorded(job, None).await.unwrap();
    std::fs::remove_file(&src).unwrap();
    let checked = check(&entry.inverse().unwrap());
    assert!(checked.runnable.is_none());
    assert_eq!(checked.skipped[0].reason, SkipReason::OriginalMissing);
}

#[tokio::test]
async fn permanent_delete_and_wipe_are_not_undoable() {
    let root = tempfile::tempdir().unwrap();
    let file = p(root.path(), "gone.txt");
    write(&file, "x");
    let job = TransferJob::new(
        TransferOperation::Delete,
        vec![file],
        PathBuf::new(),
        TransferOptions::default(),
    );
    let entry = run_recorded(job, None).await.unwrap();
    assert!(!entry.is_undoable());
    assert!(entry.inverse().is_none());
}

#[test]
fn mkdir_and_link_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let dir = p(root.path(), "made");
    std::fs::create_dir(&dir).unwrap();
    let entry = FsCommand::MakeDir { path: dir.clone() };
    write(&dir.join("inside.txt"), "x");
    let checked = check(&entry.inverse().unwrap());
    assert_eq!(checked.skipped[0].reason, SkipReason::NotEmpty);
    std::fs::remove_file(dir.join("inside.txt")).unwrap();
    assert!(check(&entry.inverse().unwrap()).runnable.is_some());

    let target = p(root.path(), "target.txt");
    write(&target, "t");
    let link = p(root.path(), "hard.txt");
    crate::fs::create_link(&target, &link, crate::fs::LinkKind::Hard).unwrap();
    let entry = FsCommand::Link {
        link: link.clone(),
        target: target.clone(),
        kind: crate::fs::LinkKind::Hard,
    };
    assert!(check(&entry.inverse().unwrap()).runnable.is_some());
    crate::fs::remove_link(&link).unwrap();
    assert_eq!(read(&target), "t", "removing the link keeps the target");
    assert!(check(&entry).runnable.is_some(), "redo can recreate it");
}

/// Trash restore needs `trash::os_limited` (Windows, Freedesktop).
#[cfg(any(
    target_os = "windows",
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
))]
#[tokio::test]
async fn trashed_file_is_restored() {
    let root = tempfile::tempdir().unwrap();
    let file = p(
        root.path(),
        &format!("pairee-undo-{}.txt", uuid::Uuid::new_v4()),
    );
    write(&file, "keep me");
    let job = TransferJob::new(
        TransferOperation::Delete,
        vec![file.clone()],
        PathBuf::new(),
        TransferOptions {
            delete_to_recycle_bin: true,
            ..TransferOptions::default()
        },
    );
    let results = run_job(job.clone()).await;
    if results.completed_files.is_empty() {
        // No usable trash for this folder (e.g. a tmpfs without .Trash).
        return;
    }
    let entry = super::from_transfer(&job, &results, None).unwrap();
    assert!(matches!(entry, FsCommand::Trash { .. }));
    restore(&entry).await;
    assert_eq!(read(&file), "keep me");
    // Clean up: the test leaves nothing in the user's trash.
    std::fs::remove_file(&file).unwrap();
}

#[cfg(any(
    target_os = "windows",
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
))]
async fn restore(entry: &FsCommand) {
    let FsCommand::Restore { paths } = runnable_inverse(entry) else {
        panic!("trash is undone by a restore");
    };
    let job = TransferJob::new(
        TransferOperation::Restore,
        paths,
        PathBuf::new(),
        TransferOptions::default(),
    );
    run_job(job).await;
}
