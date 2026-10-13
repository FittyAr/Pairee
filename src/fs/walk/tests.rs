use super::*;
use crate::fs::vfs::LocalVfs;
use std::cell::Cell;

/// Creates a link to the folder `target` at `link`; `false` where the
/// platform refuses (Windows without Developer Mode).
pub(crate) fn symlink_dir(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(target, link);
    made.is_ok()
}

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a").join("b")).unwrap();
    std::fs::write(dir.path().join("top.txt"), b"1").unwrap();
    std::fs::write(dir.path().join("a").join("b").join("deep.txt"), b"22").unwrap();
    dir
}

fn rel_paths(root: &Path, items: &[WalkEntry]) -> Vec<String> {
    let mut paths: Vec<String> = items
        .iter()
        .map(|i| {
            i.path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    paths.sort();
    paths
}

#[test]
fn walks_parents_before_children_with_depths() {
    let dir = tree();
    let items = Walker::new(&LocalVfs, dir.path(), WalkOptions::default())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        rel_paths(dir.path(), &items),
        ["a", "a/b", "a/b/deep.txt", "top.txt"]
    );
    let pos = |name: &str| items.iter().position(|i| i.path().ends_with(name)).unwrap();
    assert!(pos("a") < pos("b") && pos("b") < pos("deep.txt"));
    let deep = &items[pos("deep.txt")];
    assert_eq!((deep.depth, deep.descends, deep.entry.size), (3, false, 2));
}

#[test]
fn skip_dir_prunes_the_folder_just_returned() {
    let dir = tree();
    let mut walker = Walker::new(&LocalVfs, dir.path(), WalkOptions::default());
    let mut seen = Vec::new();
    while let Some(item) = walker.next() {
        let item = item.unwrap();
        if item.path().ends_with("a") {
            walker.skip_dir();
        }
        seen.push(item);
    }
    assert_eq!(rel_paths(dir.path(), &seen), ["a", "top.txt"]);
}

#[test]
fn an_unreadable_root_is_reported_not_swallowed() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let mut walker = Walker::new(&LocalVfs, &missing, WalkOptions::default());
    match walker.next() {
        Some(Err(WalkError::Io { path, .. })) => assert_eq!(path, missing),
        other => panic!("expected an I/O error, got {other:?}"),
    }
    assert!(walker.next().is_none());
}

#[test]
fn cancellation_stops_the_walk() {
    let dir = tree();
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() > 2
    };
    let walker = Walker::new(&LocalVfs, dir.path(), WalkOptions::default()).with_cancel(&cancelled);
    let items: Vec<_> = walker.collect();
    assert!(matches!(items.last(), Some(Err(WalkError::Cancelled))));
    assert!(items.len() <= 3);
}

#[test]
fn depth_limit_reports_too_deep_folders() {
    let dir = tree();
    let options = WalkOptions {
        max_depth: 1,
        ..WalkOptions::default()
    };
    let items: Vec<_> = Walker::new(&LocalVfs, dir.path(), options).collect();
    assert!(
        items
            .iter()
            .any(|i| matches!(i, Err(WalkError::TooDeep { path }) if path.ends_with("b")))
    );
    assert!(items.iter().flatten().all(|i| i.depth <= 1));
}

#[test]
fn link_policies_keep_skip_and_follow_with_loop_detection() {
    let dir = tree();
    let root = dir.path();
    // A link back to the root: following it would never end.
    if !symlink_dir(root, &root.join("a").join("loop")) {
        return;
    }
    let walk = |links| -> Vec<Result<WalkEntry, WalkError>> {
        Walker::new(&LocalVfs, root, WalkOptions::with_links(links)).collect()
    };

    let kept = walk(LinkPolicy::Keep);
    let link = kept
        .iter()
        .flatten()
        .find(|i| i.path().ends_with("loop"))
        .unwrap();
    assert!(link.is_kept_link() && !link.descends);

    assert!(
        walk(LinkPolicy::Skip)
            .iter()
            .flatten()
            .all(|i| !i.path().ends_with("loop"))
    );

    let followed = walk(LinkPolicy::Follow);
    assert!(
        followed
            .iter()
            .any(|i| matches!(i, Err(WalkError::Cycle { path }) if path.ends_with("loop"))),
        "{followed:?}"
    );
    // Everything else is still walked.
    assert!(
        followed
            .iter()
            .flatten()
            .any(|i| i.path().ends_with("deep.txt"))
    );
}

#[test]
fn followed_links_to_folders_are_entered() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("x.txt"), b"xyz").unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir(&root).unwrap();
    if !symlink_dir(&outside, &root.join("link")) {
        return;
    }
    let items = Walker::new(&LocalVfs, &root, WalkOptions::following(true))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rel_paths(&root, &items), ["link", "link/x.txt"]);
    assert!(items[0].followed && items[0].descends);
}
