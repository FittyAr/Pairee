//! Tempfile-tree tests of the comparison engine and the action plan.

mod apply;
mod modes;

use super::*;
use crate::fs::compare::CompareOptions;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Observer that never cancels and counts progress reports.
#[derive(Default)]
pub(super) struct Quiet {
    reports: Cell<usize>,
}

impl ScanObserver for Quiet {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn progress(&self, _: &ScanProgress) {
        self.reports.set(self.reports.get() + 1);
    }
}

struct Cancelled;

impl ScanObserver for Cancelled {
    fn is_cancelled(&self) -> bool {
        true
    }
    fn progress(&self, _: &ScanProgress) {}
}

/// Base time of every fixture file (one day ago).
fn base_time() -> SystemTime {
    SystemTime::now() - Duration::from_secs(86_400)
}

/// Writes `root/rel` with `content` and a modification time `offset_secs`
/// after [`base_time`].
pub(super) fn put(root: &Path, rel: &str, content: &str, offset_secs: u64) -> PathBuf {
    put_at(
        root,
        rel,
        content,
        base_time() + Duration::from_secs(offset_secs),
    )
}

fn put_at(root: &Path, rel: &str, content: &str, time: SystemTime) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(time)).unwrap();
    path
}

pub(super) fn opts(direction: SyncDirection) -> SyncOptions {
    SyncOptions {
        compare: CompareOptions {
            case_insensitive: false,
            ..CompareOptions::default()
        },
        direction,
        ..SyncOptions::default()
    }
}

pub(super) fn diff(left: &Path, right: &Path, options: &SyncOptions) -> Vec<SyncItem> {
    diff_trees(
        Side::local(left),
        Side::local(right),
        options,
        &Quiet::default(),
    )
    .unwrap()
}

pub(super) fn find<'a>(items: &'a [SyncItem], rel: &str) -> &'a SyncItem {
    items
        .iter()
        .find(|i| i.rel_path == Path::new(rel))
        .unwrap_or_else(|| panic!("no item {rel} in {items:#?}"))
}

pub(super) fn trees() -> (tempfile::TempDir, tempfile::TempDir) {
    (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
}

#[test]
fn recursion_reports_nested_items_and_rolls_up_folders() {
    let (l, r) = trees();
    put(l.path(), "sub/deep/x.txt", "left", 100);
    put(r.path(), "sub/deep/x.txt", "right", 0);
    put(l.path(), "same/a.txt", "a", 0);
    put(r.path(), "same/a.txt", "a", 0);
    put(l.path(), "only/one.txt", "12345", 0);
    put(l.path(), "only/nested/two.txt", "123", 0);

    let observer = Quiet::default();
    let items = diff_trees(
        Side::local(l.path()),
        Side::local(r.path()),
        &opts(SyncDirection::Both),
        &observer,
    )
    .unwrap();

    assert_eq!(find(&items, "sub/deep/x.txt").kind, DiffKind::LeftNewer);
    assert_eq!(find(&items, "sub/deep").kind, DiffKind::Differs);
    assert_eq!(find(&items, "sub").kind, DiffKind::Differs);
    assert!(find(&items, "sub").is_dir_pair());
    assert_eq!(find(&items, "sub").action, SyncAction::Skip);
    assert_eq!(find(&items, "same").kind, DiffKind::Equal);
    let only = find(&items, "only");
    assert_eq!(only.kind, DiffKind::OnlyLeft);
    assert_eq!(only.left_bytes, 8, "one-sided folder counts its files");
    assert_eq!(only.right_path, r.path().join("only"));
    assert!(
        !items
            .iter()
            .any(|i| i.rel_path.starts_with("only") && i.rel_path != Path::new("only")),
        "a one-sided folder is a single item"
    );
    assert!(observer.reports.get() >= 4, "one report per folder pair");
}

#[test]
fn tolerance_absorbs_fat_granularity_and_is_configurable() {
    let (l, r) = trees();
    let t = base_time();
    put_at(l.path(), "a.txt", "x", t + Duration::from_millis(1500));
    put_at(r.path(), "a.txt", "x", t);

    let mut options = opts(SyncDirection::LeftToRight);
    assert_eq!(
        find(&diff(l.path(), r.path(), &options), "a.txt").kind,
        DiffKind::Equal
    );
    options.compare.mtime_tolerance = Duration::from_secs(1);
    let item = find(&diff(l.path(), r.path(), &options), "a.txt").clone();
    assert_eq!(item.kind, DiffKind::LeftNewer);
    assert_eq!(item.action, SyncAction::CopyToRight);
}

#[test]
fn case_handling_pairs_names_and_keeps_each_spelling() {
    let (l, r) = trees();
    put(l.path(), "Docs/Read.ME", "x", 0);
    put(r.path(), "docs/read.me", "x", 0);

    let mut options = opts(SyncDirection::LeftToRight);
    options.compare.case_insensitive = true;
    let items = diff(l.path(), r.path(), &options);
    let file = find(&items, "Docs/Read.ME");
    assert_eq!(file.kind, DiffKind::Equal);
    assert_eq!(file.right_path, r.path().join("docs").join("read.me"));
    assert_eq!(items.len(), 2);

    options.compare.case_insensitive = false;
    let items = diff(l.path(), r.path(), &options);
    assert_eq!(find(&items, "Docs").kind, DiffKind::OnlyLeft);
    assert_eq!(find(&items, "docs").kind, DiffKind::OnlyRight);
}

#[test]
fn filters_include_exclude_and_hidden() {
    let (l, r) = trees();
    put(l.path(), "keep.txt", "x", 0);
    put(l.path(), "drop.log", "x", 0);
    put(l.path(), "target/out.txt", "x", 0);
    put(l.path(), ".hidden", "x", 0);
    put(l.path(), "src/.cache.txt", "x", 0);

    let mut options = opts(SyncDirection::LeftToRight);
    options.mask = "*.txt;!target".into();
    options.ignore_hidden = true;
    assert_eq!(options.effective_mask(), "*.txt;!target;!.*");
    let items = diff(l.path(), r.path(), &options);
    let names: Vec<_> = items.iter().map(|i| i.rel_path.clone()).collect();
    assert_eq!(names, [PathBuf::from("keep.txt"), PathBuf::from("src")]);
    assert_eq!(find(&items, "src").left_bytes, 0, "hidden file not counted");

    options.ignore_hidden = false;
    options.mask.clear();
    assert_eq!(options.effective_mask(), "");
    assert_eq!(diff(l.path(), r.path(), &options).len(), 5);
}

#[test]
fn content_hash_decides_for_same_size_files() {
    let (l, r) = trees();
    put(l.path(), "same_time.bin", "AAAA", 0);
    put(r.path(), "same_time.bin", "BBBB", 0);
    put(l.path(), "touched.bin", "CCCC", 500);
    put(r.path(), "touched.bin", "CCCC", 0);

    let mut options = opts(SyncDirection::LeftToRight);
    let items = diff(l.path(), r.path(), &options);
    assert_eq!(find(&items, "same_time.bin").kind, DiffKind::Equal);
    assert_eq!(find(&items, "touched.bin").kind, DiffKind::LeftNewer);

    options.content_hash = Some(crate::fs::transfer::options::HashAlgorithm::Blake3);
    let items = diff(l.path(), r.path(), &options);
    assert_eq!(find(&items, "same_time.bin").kind, DiffKind::Differs);
    assert_eq!(find(&items, "touched.bin").kind, DiffKind::Equal);
}

#[test]
fn file_against_folder_is_a_type_mismatch_and_skipped() {
    let (l, r) = trees();
    put(l.path(), "thing", "x", 0);
    put(r.path(), "thing/inner.txt", "x", 0);
    let items = diff(l.path(), r.path(), &opts(SyncDirection::Mirror));
    let item = find(&items, "thing");
    assert_eq!(item.kind, DiffKind::TypeMismatch);
    assert_eq!(item.action, SyncAction::Skip);
    assert_eq!(plan::allowed_actions(item), [SyncAction::Skip]);
}

#[test]
fn cancellation_stops_the_scan() {
    let (l, r) = trees();
    put(l.path(), "a.txt", "x", 0);
    let result = diff_trees(
        Side::local(l.path()),
        Side::local(r.path()),
        &SyncOptions::default(),
        &Cancelled,
    );
    assert!(matches!(result, Err(SyncError::Cancelled)));
}

#[test]
fn missing_root_is_an_io_error() {
    let (l, r) = trees();
    let missing = r.path().join("nope");
    let result = diff_trees(
        Side::local(l.path()),
        Side::local(&missing),
        &SyncOptions::default(),
        &Quiet::default(),
    );
    assert!(matches!(result, Err(SyncError::Io { path, .. }) if path == missing));
}

#[test]
fn an_archive_compares_against_a_local_folder() {
    use crate::fs::archive::ArchiveVfs;
    use crate::fs::archive::test_fixtures::write_zip;
    let (l, r) = trees();
    let zip = l.path().join("a.zip");
    write_zip(&zip, &[("same.txt", b"same"), ("only_zip.txt", b"z")]);
    put(r.path(), "same.txt", "same", 0);
    put(r.path(), "only_dir.txt", "d", 0);

    let archive = ArchiveVfs::open(zip.clone()).unwrap();
    let left = Side {
        vfs: &archive,
        root: &zip,
    };
    let options = SyncOptions {
        content_hash: Some(crate::fs::transfer::options::HashAlgorithm::Blake3),
        ..SyncOptions::default()
    };
    let items = diff_trees(left, Side::local(r.path()), &options, &Quiet::default()).unwrap();
    assert_eq!(find(&items, "same.txt").kind, DiffKind::Equal);
    assert_eq!(find(&items, "only_zip.txt").kind, DiffKind::OnlyLeft);
    assert_eq!(find(&items, "only_dir.txt").kind, DiffKind::OnlyRight);
}
