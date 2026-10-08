//! Folder sizes, the disk usage view, folder compare and synchronize
//! (left to right plan applied through the Transfer Engine).

use crate::app::state::{ActivePanel, PopupType};
use crate::test_harness::Harness;

fn tree() -> Harness {
    let h = Harness::new();
    h.write("work/left/media/clip.bin", vec![0u8; 3000]);
    h.write("work/left/notes.txt", [0u8; 10]);
    h
}

#[test]
fn folder_sizes_in_the_panel() {
    let mut h = tree();
    h.reread().keys("@panel_view_full @calculate_folder_sizes");
    let media = h.left().join("media");
    let size = h.state.get_active_panel().dir_sizes.get(&media).cloned();
    assert!(size.is_some(), "size computed");
    h.assert_screen("2.9 KB");
}

#[test]
fn disk_usage_view_navigates_and_deletes() {
    let mut h = tree();
    h.reread().keys("@disk_usage");
    assert!(matches!(h.state.dialogs.top(), Some(PopupType::DiskUsage)));
    h.assert_screen("media/");
    h.assert_screen("notes.txt");
    h.keys("Enter");
    assert_eq!(h.state.disk_usage.current_path(), h.left().join("media"));
    h.assert_screen("clip.bin");
    // Delete goes through the regular confirmation, over the view.
    h.keys("Delete");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConfirmDelete { .. })
    ));
    h.keys("Enter");
    h.wait_until("delete from the view", |h| {
        !h.exists("work/left/media/clip.bin")
    });
    assert!(matches!(h.state.dialogs.top(), Some(PopupType::DiskUsage)));
    h.assert_no_screen("clip.bin");
    h.keys("Backspace Esc");
    assert!(h.state.dialogs.is_none());
}

/// left: new.txt, both.txt (newer); right: both.txt (older), extra.txt.
fn two_sides() -> Harness {
    let h = Harness::new();
    let older = filetime::FileTime::from_unix_time(1_600_000_000, 0);
    h.write("work/left/new.txt", "new");
    h.write("work/left/both.txt", "left version");
    let right = h.write("work/right/both.txt", "old");
    filetime::set_file_mtime(&right, older).unwrap();
    h.write("work/right/extra.txt", "extra");
    h
}

#[test]
fn compare_selects_the_differences() {
    let mut h = two_sides();
    h.reread().keys("@compare_folder");
    h.wait_until("compare result", |h| {
        matches!(
            h.state.dialogs.top(),
            Some(PopupType::CompareFoldersResult { .. })
        )
    });
    let left = h.state.panels.side(ActivePanel::Left);
    assert!(left.selected_paths.contains(&h.left().join("new.txt")));
    assert!(left.selected_paths.contains(&h.left().join("both.txt")));
}

#[test]
fn sync_left_to_right_applies_the_plan() {
    let mut h = two_sides();
    h.reread().keys("@sync_dirs");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::SyncDirs(_))
    ));
    // Default direction left → right; Enter on the mask runs the compare.
    h.keys("Down*3 Enter");
    h.wait_until("review", |h| match h.state.dialogs.top() {
        Some(PopupType::SyncDirs(dialog)) => dialog.review.is_some(),
        _ => false,
    });
    h.assert_screen("new.txt");
    h.keys("Enter y");
    h.wait_until("sync jobs", |h| {
        h.read("work/right/both.txt") == b"left version"
    });
    assert_eq!(h.read("work/right/new.txt"), b"new");
    assert!(
        h.exists("work/right/extra.txt"),
        "left to right keeps extras"
    );
}
