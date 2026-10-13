use super::*;
use crate::fs::transfer::job::TransferJob;
use crate::fs::vfs::LocalVfs;
use crate::fs::walk::tests::symlink_dir;

fn control() -> JobControl {
    let (tx, _rx) = crate::fs::transfer::events::EventSender::channel();
    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![],
        PathBuf::new(),
        TransferOptions::default(),
    );
    JobControl::for_job(&job, tx)
}

fn run(
    roots: &[(PathBuf, PathBuf)],
    operation: TransferOperation,
    options: &TransferOptions,
    dst_links: bool,
) -> ScanOutcome {
    let request = ScanRequest {
        roots,
        operation,
        options,
        src_fs: &LocalVfs,
        dst_fs: &LocalVfs,
        same_fs: true,
        dst_links,
    };
    scan(&request, &control()).unwrap()
}

fn tree(root: &Path) {
    std::fs::create_dir_all(root.join("a").join("sub")).unwrap();
    std::fs::write(root.join("a").join("one.txt"), b"1").unwrap();
    std::fs::write(root.join("a").join("sub").join("two.txt"), b"22").unwrap();
}

#[test]
fn maps_files_and_folders_below_the_target() {
    let dir = tempfile::tempdir().unwrap();
    tree(dir.path());
    let src = dir.path().join("a");
    let dst = dir.path().join("b");
    let out = run(
        &[(src.clone(), dst.clone())],
        TransferOperation::Move,
        &TransferOptions::default(),
        true,
    );
    assert_eq!(out.total_bytes, 3);
    assert_eq!(out.mappings.len(), 2);
    assert!(
        out.mappings
            .iter()
            .any(|(s, d, _)| s.ends_with("two.txt") && *d == dst.join("sub").join("two.txt"))
    );
    assert_eq!(out.dst_dirs, vec![dst.clone(), dst.join("sub")]);
    assert_eq!(out.dirs_to_delete, vec![src.clone(), src.join("sub")]);
    assert!(out.refused.is_empty());
}

/// Regression (FS-05): copying a folder into itself used to nest copies
/// until the path got too long.
#[test]
fn a_folder_copied_into_itself_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    tree(dir.path());
    let src = dir.path().join("a");
    for dst in [src.join("sub").join("a"), src.clone()] {
        let out = run(
            &[(src.clone(), dst)],
            TransferOperation::Copy,
            &TransferOptions::default(),
            true,
        );
        assert!(out.mappings.is_empty());
        assert_eq!(out.refused.len(), 1);
        assert_eq!(out.refused[0].src, src);
    }
}

/// Regression (FS-06): a delete with "follow links" set must not delete
/// what a link inside the tree points to.
#[test]
fn a_delete_never_follows_links() {
    let dir = tempfile::tempdir().unwrap();
    tree(dir.path());
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), b"keep").unwrap();
    let link = dir.path().join("a").join("link");
    if !symlink_dir(&outside, &link) {
        return;
    }
    let options = TransferOptions {
        follow_symlinks: true,
        ..TransferOptions::default()
    };
    let out = run(
        &[(dir.path().join("a"), PathBuf::new())],
        TransferOperation::Delete,
        &options,
        true,
    );
    assert!(out.mappings.iter().any(|(s, _, _)| *s == link));
    assert!(
        out.mappings
            .iter()
            .all(|(s, _, _)| !s.starts_with(&outside) && !s.ends_with("keep.txt"))
    );
}

/// Where links cannot be recreated (SFTP), a link to a folder is skipped
/// with a warning instead of failing the whole job.
#[test]
fn folder_links_are_skipped_where_links_cannot_be_made() {
    let dir = tempfile::tempdir().unwrap();
    tree(dir.path());
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    if !symlink_dir(&outside, &dir.path().join("a").join("link")) {
        return;
    }
    let out = run(
        &[(dir.path().join("a"), dir.path().join("b"))],
        TransferOperation::Copy,
        &TransferOptions::default(),
        false,
    );
    assert_eq!(out.mappings.len(), 2);
    assert_eq!(out.skipped.len(), 1);
    assert!(out.skipped[0].src.ends_with("link"));
}

#[test]
fn missing_sources_are_refused_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(
        &[(dir.path().join("missing"), dir.path().join("x"))],
        TransferOperation::Copy,
        &TransferOptions::default(),
        true,
    );
    assert_eq!(out.refused.len(), 1);
}
