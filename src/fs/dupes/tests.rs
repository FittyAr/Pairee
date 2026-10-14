use super::*;
use crate::fs::vfs::LocalVfs;
use std::cell::Cell;
use std::fs;
use std::path::Path;

const NONE: DupeControl<'static> = DupeControl {
    cancelled: &|| false,
    progress: &|_| {},
};

fn write(root: &Path, rel: &str, data: &[u8]) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, data).unwrap();
    path
}

/// Same size everywhere, different only past the partial pass (so the
/// full pass has to tell them apart).
fn big(tail: u8) -> Vec<u8> {
    let mut data = vec![7u8; 100_000];
    data[99_999] = tail;
    data
}

fn options() -> DupeOptions {
    DupeOptions {
        partial_bytes: 4096,
        ..DupeOptions::default()
    }
}

#[test]
fn groups_equal_files_and_tells_near_misses_apart() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let a = write(root, "a.txt", b"same");
    let b = write(root, "sub/b.txt", b"same");
    write(root, "c.txt", b"diff");
    write(root, "lonely.txt", b"only one of its size");
    let big1 = write(root, "big1.bin", &big(1));
    let big2 = write(root, "deep/er/big2.bin", &big(1));
    write(root, "big3.bin", &big(2));
    write(root, "empty1", b"");
    write(root, "empty2", b"");

    let report = find_duplicates(&LocalVfs, &[root.to_path_buf()], &options(), &NONE);
    assert!(!report.partial);
    let found: Vec<Vec<PathBuf>> = report.groups.iter().map(|g| g.files.clone()).collect();
    let mut bigs = vec![big1, big2];
    bigs.sort();
    let mut smalls = vec![a, b];
    smalls.sort();
    assert_eq!(found, [bigs, smalls], "most wasted space first");
    assert_eq!(report.groups[0].wasted(), 100_000);
    assert_eq!(report.groups[1].size, 4);
}

#[test]
fn overlapping_roots_count_each_file_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "x/a.txt", b"same");
    write(root, "x/b.txt", b"other");
    let roots = [root.to_path_buf(), root.join("x")];
    let report = find_duplicates(&LocalVfs, &roots, &options(), &NONE);
    assert!(report.groups.is_empty(), "{:?}", report.groups);
}

#[test]
fn cancelling_returns_a_partial_report() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for i in 0..10 {
        write(root, &format!("f{i}.txt"), b"same");
    }
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() > 5
    };
    let ctl = DupeControl {
        cancelled: &cancelled,
        progress: &|_| {},
    };
    let report = find_duplicates(&LocalVfs, &[root.to_path_buf()], &options(), &ctl);
    assert!(report.partial);
    assert!(report.groups.is_empty());
}

#[test]
fn a_missing_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("gone");
    let report = find_duplicates(&LocalVfs, std::slice::from_ref(&gone), &options(), &NONE);
    assert!(report.partial);
    assert_eq!(report.unreadable, [gone]);
}

#[cfg(unix)]
#[test]
fn hard_links_and_symlinks_are_not_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let a = write(root, "a.txt", b"same");
    fs::hard_link(&a, root.join("hard.txt")).unwrap();
    std::os::unix::fs::symlink(&a, root.join("soft.txt")).unwrap();
    let report = find_duplicates(&LocalVfs, &[root.to_path_buf()], &options(), &NONE);
    assert!(report.groups.is_empty(), "{:?}", report.groups);
}

/// With links followed, files below a linked folder are searched, the
/// linked copy of a file is not its own duplicate and a link looping back
/// to an ancestor does not repeat the search.
#[test]
fn followed_links_reach_more_files_once_and_survive_loops() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let a = write(&root, "a.bin", &big(1));
    write(elsewhere.path(), "far/b.bin", &big(1));
    let linked =
        crate::fs::walk::tests::symlink_dir(&elsewhere.path().join("far"), &root.join("far"));
    let looped = crate::fs::walk::tests::symlink_dir(&root, &root.join("loop"));
    if !(linked && looped) {
        return; // Windows without the right to create links.
    }
    let skip = find_duplicates(&LocalVfs, std::slice::from_ref(&root), &options(), &NONE);
    assert!(
        skip.groups.is_empty(),
        "links not followed: {:?}",
        skip.groups
    );

    let follow = DupeOptions {
        follow_links: true,
        ..options()
    };
    let report = find_duplicates(&LocalVfs, std::slice::from_ref(&root), &follow, &NONE);
    assert_eq!(report.groups.len(), 1, "{:?}", report.groups);
    let files = &report.groups[0].files;
    assert_eq!(files.len(), 2, "each file once: {files:?}");
    assert!(files.contains(&a));
}
