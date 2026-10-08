//! Single-item file operations: make folder, rename, delete (permanent and
//! to the trash) and undo / redo of the journal.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

const KEYMAPS: [&str; 2] = ["norton", "vscode"];

/// Clears the focused one-line field of a dialog.
fn clear_field(h: &mut Harness, len: usize) {
    h.keys(&format!("End Backspace*{len}"));
}

#[test]
fn make_folder() {
    for keymap in KEYMAPS {
        let mut h = Harness::builder().keymap(keymap).build();
        h.keys("@mkdir");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::MkDirPrompt { .. })),
            "{keymap}"
        );
        h.text("nueva carpeta").keys("Enter");
        assert!(h.state.dialogs.is_none(), "{keymap}");
        assert!(h.left().join("nueva carpeta").is_dir(), "{keymap}");
        h.assert_screen("nueva carpeta");
    }
}

#[test]
fn rename_then_undo_and_redo() {
    for keymap in KEYMAPS {
        let mut h = Harness::builder().keymap(keymap).build();
        h.write("work/left/old.txt", "x");
        h.reread().focus("old.txt").keys("@rename");
        h.assert_screen("old.txt");
        clear_field(&mut h, "old.txt".len());
        h.text("new.txt").keys("Enter");
        assert!(h.exists("work/left/new.txt"), "{keymap}");
        assert!(!h.exists("work/left/old.txt"), "{keymap}");

        h.keys("@undo_file_operation");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::ConfirmUndo { .. })),
            "{keymap}"
        );
        h.keys("Enter");
        h.wait_until("undo of the rename", |h| h.exists("work/left/old.txt"));
        assert!(!h.exists("work/left/new.txt"));

        h.keys("@redo_file_operation Enter");
        h.wait_until("redo of the rename", |h| h.exists("work/left/new.txt"));
        assert!(!h.exists("work/left/old.txt"));
    }
}

#[test]
fn permanent_delete_asks_first() {
    for keymap in KEYMAPS {
        let mut h = Harness::builder().keymap(keymap).build();
        h.write("work/left/doomed.txt", "x");
        h.reread().focus("doomed.txt").keys("@delete");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::ConfirmDelete { .. })),
            "{keymap}"
        );
        h.assert_screen("doomed.txt");
        // Esc and the Cancel button keep the file.
        h.keys("Esc");
        assert!(h.exists("work/left/doomed.txt"));
        h.keys("@delete Right Enter");
        assert!(h.exists("work/left/doomed.txt"));
        assert!(h.state.dialogs.is_none());
        // Enter on [Delete] removes it.
        h.keys("@delete Enter");
        h.wait_until("delete", |h| !h.exists("work/left/doomed.txt"));
        h.reread();
        assert!(!h.names().contains(&"doomed.txt".to_string()));
    }
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
#[test]
fn delete_to_trash_then_undo_and_redo() {
    let mut h = Harness::builder()
        .settings(|s| s.delete_to_recycle_bin = true)
        .build();
    if !crate::fs::journal::trash::trash_round_trips_in(&h.left()) {
        eprintln!("skipped: no usable trash for {}", h.left().display());
        return;
    }
    let name = format!("pairee-smoke-{}.txt", uuid::Uuid::new_v4());
    let rel = format!("work/left/{name}");
    h.write(&rel, "keep me");
    h.reread().focus(&name).keys("@delete Enter");
    h.wait_until("trash", |h| !h.exists(&rel) || h.state.dialogs.is_some());
    if h.exists(&rel) {
        // No usable trash for this folder (e.g. a tmpfs without .Trash).
        return;
    }
    h.keys("@undo_file_operation Enter");
    h.wait_until("restore from the trash", |h| h.exists(&rel));
    assert_eq!(h.read(&rel), b"keep me");

    h.keys("@redo_file_operation Enter");
    h.wait_until("trash again", |h| !h.exists(&rel));
    // Leave nothing in the user's trash.
    h.keys("@undo_file_operation Enter");
    h.wait_until("restore again", |h| h.exists(&rel));
}
