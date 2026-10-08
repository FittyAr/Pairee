use super::apply_listing;
use super::filter::{git_pathspec_for, partition_entries_by_mask, skip_auto_update};
use super::listing::PanelListing;
use crate::app::state::{ActivePanel, AppState, PanelState};
use crate::fs::FileEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn entry(name: &str, dir: &Path, is_dir: bool) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        path: dir.join(name),
        is_dir,
        is_symlink: false,
        size: 0,
        modified: None,
    }
}

fn listing(path: &Path, names: &[&str]) -> PanelListing {
    PanelListing {
        path: path.to_path_buf(),
        entries: Ok(names.iter().map(|n| entry(n, path, false)).collect()),
        git: None,
        free_space: Some(1),
        attrs: HashMap::new(),
        show_hidden: false,
    }
}

fn names(panel: &PanelState) -> Vec<&str> {
    panel.entries.iter().map(|e| e.name.as_str()).collect()
}

fn temp_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("b.txt"), b"b").unwrap();
    std::fs::write(dir.path().join("c.txt"), b"c").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    dir
}

#[test]
fn partition_keeps_dotdot_then_matches() {
    let root = PathBuf::from("/");
    let entries = vec![
        entry("..", &root, true),
        entry("rust_code.rs", &root, false),
        entry("other_file.txt", &root, false),
        entry("rust_dir", &root, true),
    ];
    let partitioned = partition_entries_by_mask(entries, "rust");
    let got: Vec<_> = partitioned.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(got, ["..", "rust_code.rs", "rust_dir", "other_file.txt"]);
}

#[test]
fn natural_collation_enables_numeric_sort() {
    let mut state = AppState::new(".".into(), ".".into());
    state.sorting_collation = "linguistic".into();
    assert!(!state.natural_sort());
    state.sorting_collation = "natural".into();
    assert!(state.natural_sort());
}

#[test]
fn limit_only_applies_to_same_directory_auto_refresh() {
    assert!(skip_auto_update(10, 50, false));
    assert!(
        !skip_auto_update(10, 50, true),
        "new dir / forced reread loads"
    );
    assert!(!skip_auto_update(0, 50, false), "0 disables the limit");
    assert!(!skip_auto_update(10, 5, false));
}

#[test]
fn refresh_panel_lists_directory_without_runtime() {
    let dir = temp_tree();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    state.refresh_panel(ActivePanel::Left, false, true);
    let left = &state.panels.left;
    assert!(!left.is_loading());
    assert!(names(left).contains(&"b.txt"));
    assert!(names(left).contains(&"sub"));
    assert_eq!(left.listed_path.as_deref(), Some(dir.path()));
    assert!(
        state.panels.right.entries.is_empty(),
        "only one side reread"
    );
}

#[tokio::test]
async fn refresh_panel_runs_in_background_and_applies_result() {
    let dir = temp_tree();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    state.refresh_panel(ActivePanel::Right, false, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.panels.right.is_loading() && Instant::now() < deadline {
        state.poll_panel_listings();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(names(&state.panels.right).contains(&"c.txt"));
}

#[test]
fn refresh_keeps_cursor_on_same_entry() {
    let dir = PathBuf::from("/data");
    let mut panel = PanelState::new(dir.clone());
    apply_listing(&mut panel, listing(&dir, &["b", "c", "d"]));
    panel.cursor_index = 1; // "c"
    apply_listing(&mut panel, listing(&dir, &["a", "b", "c", "d"]));
    assert_eq!(panel.entries[panel.cursor_index].name, "c");
}

#[test]
fn pending_focus_positions_cursor_after_ascending() {
    let dir = PathBuf::from("/data");
    let mut panel = PanelState::new(dir.clone());
    panel.pending_focus = Some("c".into());
    apply_listing(&mut panel, listing(&dir, &["a", "b", "c"]));
    assert_eq!(panel.cursor_index, 2);
    assert!(panel.pending_focus.is_none());
}

#[test]
fn listing_for_another_directory_is_ignored() {
    let mut panel = PanelState::new(PathBuf::from("/new"));
    apply_listing(&mut panel, listing(Path::new("/old"), &["x"]));
    assert!(panel.entries.is_empty());
    assert!(panel.listed_path.is_none());
}

#[test]
fn selection_of_vanished_entries_is_pruned() {
    let dir = PathBuf::from("/data");
    let mut panel = PanelState::new(dir.clone());
    apply_listing(&mut panel, listing(&dir, &["a", "b"]));
    for p in [dir.join("a"), dir.join("b")] {
        panel.selected_paths.insert(p.clone());
        panel.selection_order.push(p);
    }
    apply_listing(&mut panel, listing(&dir, &["b"]));
    assert_eq!(panel.selection_order, vec![dir.join("b")]);
    assert!(!panel.selected_paths.contains(&dir.join("a")));
    assert_eq!(panel.free_space, Some(1));
}

#[test]
fn failed_listing_keeps_previous_entries() {
    let dir = PathBuf::from("/data");
    let mut panel = PanelState::new(dir.clone());
    apply_listing(&mut panel, listing(&dir, &["a"]));
    let mut failed = listing(&dir, &[]);
    failed.entries = Err("denied".into());
    apply_listing(&mut panel, failed);
    assert_eq!(names(&panel), ["a"]);
}

#[test]
fn pathspec_is_relative_to_workdir() {
    let root = Path::new("/repo/");
    assert_eq!(git_pathspec_for(root, Path::new("/repo")), None);
    assert_eq!(
        git_pathspec_for(root, Path::new("/repo/src/app")).as_deref(),
        Some("src/app")
    );
    assert_eq!(git_pathspec_for(root, Path::new("/elsewhere")), None);
    assert_eq!(git_pathspec_for(root, Path::new("/repo2/x")), None);
    assert_eq!(git_pathspec_for(root, Path::new("/repo/we[ird]")), None);
}

#[test]
fn pathspec_matches_case_insensitively() {
    assert_eq!(
        git_pathspec_for(Path::new("C:/Code/Repo/"), Path::new("c:\\code\\repo\\Src")).as_deref(),
        Some("Src")
    );
}
