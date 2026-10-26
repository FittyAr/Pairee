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
            matches!(h.state.dialogs.top(), Some(PopupType::MkDirPrompt(_))),
            "{keymap}"
        );
        h.text("nueva carpeta").keys("Enter");
        assert!(h.state.dialogs.is_empty(), "{keymap}");
        assert!(h.left().join("nueva carpeta").is_dir(), "{keymap}");
        h.assert_screen("nueva carpeta");
    }
}

/// UI-03: "Process multiple names" makes one folder per `;`-separated name.
#[test]
fn make_several_folders_at_once() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    h.text("one; two").keys("Down Space Enter");
    assert!(h.state.dialogs.is_empty());
    assert!(h.left().join("one").is_dir());
    assert!(h.left().join("two").is_dir());
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
            matches!(h.state.dialogs.top(), Some(PopupType::ConfirmUndo(_))),
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
            matches!(h.state.dialogs.top(), Some(PopupType::ConfirmDelete(_))),
            "{keymap}"
        );
        h.assert_screen("doomed.txt");
        // Esc and the Cancel button keep the file.
        h.keys("Esc");
        assert!(h.exists("work/left/doomed.txt"));
        h.keys("@delete Right Enter");
        assert!(h.exists("work/left/doomed.txt"));
        assert!(h.state.dialogs.is_empty());
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
    h.wait_until("trash", |h| !h.exists(&rel) || h.state.dialogs.is_open());
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

/// Deleting a folder the user may not remove (its parent is read-only)
/// says which path was refused and why, and offers to retry with
/// administrator rights; Esc leaves everything in place. Root ignores the
/// permissions, so the test skips as root.
#[cfg(unix)]
#[test]
fn a_refused_delete_says_why_and_offers_the_elevated_retry() {
    use std::os::unix::fs::PermissionsExt;
    if crate::fs::is_elevated() {
        return;
    }
    let mut h = Harness::new();
    h.write("work/left/locked/victim/f.txt", "x");
    let locked = h.left().join("locked");
    h.reread().focus("locked").keys("Enter");
    h.wait_until("inside locked", |h| {
        h.names().contains(&"victim".to_string())
    });
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }
    let _restore = Restore(locked.clone());

    h.focus("victim").keys("@delete Enter");
    h.wait_until("the refusal", |h| h.state.dialogs.is_open());
    match h.state.dialogs.top() {
        Some(PopupType::ConfirmRetryAsAdmin(retry)) => {
            assert_eq!(retry.paths, [locked.join("victim")]);
            let crate::app::state::AdminOpKind::Delete { reason } = &retry.op_kind else {
                panic!("not a delete: {:?}", retry.op_kind);
            };
            assert!(reason.contains("victim"), "{reason}");
            assert!(reason.contains("ermission denied"), "{reason}");
        }
        other => panic!("expected the elevated retry, got {other:?}"),
    }
    h.assert_screen("victim");
    h.keys("Esc");
    assert!(h.state.dialogs.is_empty());
    assert!(locked.join("victim").join("f.txt").exists());
}
