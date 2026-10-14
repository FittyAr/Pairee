use super::*;
use crate::fs::vfs::{Capabilities, LocalVfs, Vfs, VfsEntry};
use std::cell::Cell;
use std::fs;
use std::io;
use std::path::Path;

/// No cancellation, no progress.
const NONE: ScanControl<'static> = ScanControl {
    cancelled: &|| false,
    progress: &|_| {},
};

fn write(path: &Path, len: usize) {
    fs::write(path, vec![b'x'; len]).expect("write test file");
}

/// root/{a.txt(10), sub/{b.txt(20), deep/{c.txt(30)}}, empty/}
fn sample_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write(&root.join("a.txt"), 10);
    fs::create_dir_all(root.join("sub/deep")).unwrap();
    fs::create_dir(root.join("empty")).unwrap();
    write(&root.join("sub/b.txt"), 20);
    write(&root.join("sub/deep/c.txt"), 30);
    dir
}

fn totals(root: &Path) -> DirSize {
    scan(&LocalVfs, root, false, &NONE).size
}

#[test]
fn sums_files_and_counts_dirs() {
    let dir = sample_tree();
    let size = totals(dir.path());
    assert_eq!(
        size,
        DirSize {
            bytes: 60,
            files: 3,
            dirs: 3,
            partial: false
        }
    );
}

#[test]
fn totals_only_scan_keeps_no_children() {
    let dir = sample_tree();
    let node = scan(&LocalVfs, dir.path(), false, &NONE);
    assert!(node.children.is_empty());
}

#[test]
fn tree_children_sorted_largest_first() {
    let dir = sample_tree();
    let node = scan(&LocalVfs, dir.path(), true, &NONE);
    let names: Vec<_> = node.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["sub", "a.txt", "empty"]);
    let sub = node.descend(&["sub"]).unwrap();
    assert_eq!(sub.size.bytes, 50);
    assert_eq!(sub.size.dirs, 1);
    assert_eq!(
        node.descend(&["sub", "deep", "c.txt"]).unwrap().size.bytes,
        30
    );
    assert!(node.descend(&["missing"]).is_none());
}

#[test]
fn remove_subtracts_from_every_ancestor() {
    let dir = sample_tree();
    let mut node = scan(&LocalVfs, dir.path(), true, &NONE);
    let removed = node.remove(&["sub", "deep"]).unwrap();
    assert_eq!(removed.bytes, 30);
    assert_eq!(removed.dirs, 1);
    assert_eq!(node.size.bytes, 30);
    assert_eq!(node.size.files, 2);
    assert_eq!(node.size.dirs, 2);
    assert_eq!(node.descend(&["sub"]).unwrap().size.bytes, 20);
    assert!(node.remove(&["nope"]).is_none());
    assert!(node.remove::<&str>(&[]).is_none());
}

#[test]
fn percent_of_total() {
    let node = DuNode {
        size: DirSize {
            bytes: 25,
            ..DirSize::default()
        },
        ..DuNode::default()
    };
    assert!((node.percent_of(100) - 25.0).abs() < f64::EPSILON);
    assert_eq!(node.percent_of(0), 0.0);
}

#[test]
fn cancellation_stops_and_marks_partial() {
    let dir = sample_tree();
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() > 2
    };
    let ctl = ScanControl {
        cancelled: &cancelled,
        progress: &|_| {},
    };
    let node = scan(&LocalVfs, dir.path(), true, &ctl);
    assert!(node.size.partial);
    assert!(node.size.bytes < 60);
}

#[test]
fn cancelled_before_start_reads_nothing() {
    let dir = sample_tree();
    let ctl = ScanControl {
        cancelled: &|| true,
        progress: &|_| {},
    };
    let node = scan(&LocalVfs, dir.path(), true, &ctl);
    assert!(node.size.partial);
    assert_eq!(node.size.bytes, 0);
}

#[test]
fn final_progress_matches_totals() {
    let dir = sample_tree();
    let last = std::sync::Mutex::new(ScanProgress::default());
    let report = |p: &ScanProgress| *last.lock().unwrap() = p.clone();
    let ctl = ScanControl {
        cancelled: &|| false,
        progress: &report,
    };
    scan(&LocalVfs, dir.path(), false, &ctl);
    let last = last.lock().unwrap();
    assert_eq!((last.bytes, last.files, last.dirs), (60, 3, 3));
}

#[test]
fn missing_root_is_partial() {
    let dir = tempfile::tempdir().unwrap();
    let size = totals(&dir.path().join("gone"));
    assert!(size.partial);
    assert_eq!(size.bytes, 0);
}

/// A source with an unreadable subdirectory and an unreadable entry.
#[derive(Debug)]
struct FlakySource;

impl Vfs for FlakySource {
    fn capabilities(&self) -> Capabilities {
        Capabilities::READ_ONLY
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let entry = |name: &str| VfsEntry {
            name: name.into(),
            path: dir.join(name),
            ..VfsEntry::default()
        };
        match dir.to_str() {
            Some("root") => Ok(vec![
                VfsEntry {
                    size: 7,
                    ..entry("ok.bin")
                },
                VfsEntry {
                    is_dir: true,
                    ..entry("locked")
                },
                VfsEntry {
                    unreadable: true,
                    ..entry("broken")
                },
            ]),
            _ => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        }
    }

    fn stat(&self, _path: &Path) -> io::Result<VfsEntry> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

#[test]
fn permission_errors_give_partial_result() {
    let node = scan(&FlakySource, Path::new("root"), true, &NONE);
    assert!(node.size.partial);
    assert_eq!(node.size.bytes, 7);
    assert_eq!(node.size.files, 1);
    // The unreadable entry is not listed; the locked dir is, marked partial.
    let names: Vec<_> = node.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["ok.bin", "locked"]);
    assert!(node.children[1].size.partial);
}

/// A flat source with a link and two hard links of one file.
#[derive(Debug)]
struct LinkSource;

impl Vfs for LinkSource {
    fn capabilities(&self) -> Capabilities {
        Capabilities::READ_ONLY
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let entry = |name: &str, size| VfsEntry {
            name: name.into(),
            path: dir.join(name),
            size,
            ..VfsEntry::default()
        };
        Ok(vec![
            VfsEntry {
                is_symlink: true,
                link_size: Some(3),
                ..entry("link", 1000)
            },
            VfsEntry {
                hard_link: Some((1, 2)),
                ..entry("one", 40)
            },
            VfsEntry {
                hard_link: Some((1, 2)),
                ..entry("two", 40)
            },
        ])
    }

    fn stat(&self, _path: &Path) -> io::Result<VfsEntry> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

#[test]
fn links_count_their_own_size_and_hard_links_once() {
    let node = scan(&LinkSource, Path::new("root"), true, &NONE);
    assert_eq!((node.size.bytes, node.size.files), (43, 3));
    assert_eq!(node.children.len(), 3);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    #[test]
    fn symlink_loops_are_not_followed() {
        let dir = sample_tree();
        let root = dir.path();
        symlink(root, root.join("sub/loop")).unwrap();
        symlink(root.join("a.txt"), root.join("link.txt")).unwrap();
        let size = totals(root);
        let link_len = |p: PathBuf| fs::symlink_metadata(p).unwrap().len();
        let links = link_len(root.join("sub/loop")) + link_len(root.join("link.txt"));
        assert_eq!(size.bytes, 60 + links);
        assert_eq!(size.files, 5);
        assert_eq!(size.dirs, 3);
    }

    #[test]
    fn hard_links_counted_once() {
        let dir = sample_tree();
        let root = dir.path();
        fs::hard_link(root.join("a.txt"), root.join("sub/a-again.txt")).unwrap();
        fs::hard_link(root.join("a.txt"), root.join("empty/a-third.txt")).unwrap();
        let size = totals(root);
        assert_eq!(size.bytes, 60);
        assert_eq!(size.files, 5);
    }

    #[test]
    fn unreadable_directory_is_partial() {
        use std::os::unix::fs::PermissionsExt;
        let dir = sample_tree();
        let locked = dir.path().join("sub/deep");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(&locked).is_ok(); // e.g. running as root
        let size = totals(dir.path());
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if !readable {
            assert!(size.partial);
            assert_eq!(size.bytes, 30);
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_directory_symlink_not_followed() {
    let dir = sample_tree();
    let root = dir.path();
    // Creating symlinks needs developer mode or admin rights; skip otherwise.
    if std::os::windows::fs::symlink_dir(root, root.join("sub/loop")).is_err() {
        return;
    }
    let size = totals(root);
    assert_eq!(size.bytes, 60);
    assert_eq!(size.dirs, 3);
    assert_eq!(size.files, 4);
}

/// The local disk, except that reading the entries of `sub` fails after
/// the first ones (as `read_dir` iteration errors do).
#[derive(Debug)]
struct Gappy;

impl Vfs for Gappy {
    fn capabilities(&self) -> Capabilities {
        Capabilities::READ_ONLY
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        LocalVfs.list(dir)
    }

    fn list_reporting(&self, dir: &Path) -> io::Result<(Vec<VfsEntry>, Option<io::Error>)> {
        let entries = LocalVfs.list(dir)?;
        let gap = dir
            .ends_with("sub")
            .then(|| io::Error::other("entry vanished"));
        Ok((entries, gap))
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        LocalVfs.stat(path)
    }
}

#[test]
fn entries_a_listing_left_out_mark_the_totals_partial() {
    let dir = sample_tree();
    let tree = scan(&Gappy, dir.path(), true, &NONE);
    // What could be read is still counted.
    assert_eq!(tree.size.bytes, 60);
    assert!(tree.size.partial, "{:?}", tree.size);
    let sub = tree.children.iter().find(|c| c.name == "sub").unwrap();
    assert!(sub.size.partial);
    let empty = tree.children.iter().find(|c| c.name == "empty").unwrap();
    assert!(!empty.size.partial);
}

#[test]
fn local_listings_report_no_gap_on_a_healthy_folder() {
    let dir = sample_tree();
    let (entries, gap) = LocalVfs.list_reporting(dir.path()).unwrap();
    assert_eq!(entries.len(), 3);
    assert!(gap.is_none());
}
