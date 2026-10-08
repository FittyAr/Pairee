//! Copy / move through the shared transfer dialog (with the conflict
//! prompt) and the multi-rename tool.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

#[test]
fn copy_move_and_conflict_prompt() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        h.write("work/left/a.txt", "first");
        h.write("work/left/b.txt", "move me");
        h.reread().focus("a.txt").keys("@copy");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::TransferPrompt(_))),
            "{keymap}"
        );
        h.assert_screen("a.txt");
        h.keys("Enter");
        h.wait_until("copy", |h| h.exists("work/right/a.txt"));
        assert_eq!(h.read("work/right/a.txt"), b"first");

        // Copying again over a changed file asks what to do.
        h.write("work/left/a.txt", "second");
        h.focus("a.txt").keys("@copy Enter");
        h.wait_until("conflict prompt", |h| {
            h.state
                .transfer
                .as_ref()
                .is_some_and(|t| t.active_conflict_info.is_some())
        });
        assert!(matches!(
            h.state.dialogs.top(),
            Some(PopupType::TransferPanel)
        ));
        h.keys("o");
        h.wait_until("overwrite", |h| h.read("work/right/a.txt") == b"second");

        h.keys("Esc");
        h.reread().focus("b.txt").keys("@move Enter");
        h.wait_until("move", |h| {
            h.exists("work/right/b.txt") && !h.exists("work/left/b.txt")
        });
        assert_eq!(h.read("work/right/b.txt"), b"move me");
    }
}

#[test]
fn conflict_skip_keeps_the_target() {
    let mut h = Harness::new();
    h.write("work/left/same.txt", "new");
    h.write("work/right/same.txt", "old");
    h.reread().focus("same.txt").keys("@copy Enter");
    h.wait_until("conflict prompt", |h| {
        h.state
            .transfer
            .as_ref()
            .is_some_and(|t| t.active_conflict_info.is_some())
    });
    h.keys("s");
    h.settle();
    assert_eq!(h.read("work/right/same.txt"), b"old");
}

#[test]
fn multi_rename_with_mask_preview_and_run() {
    let mut h = Harness::new();
    for name in ["b.jpg", "a.jpg", "c.jpg"] {
        h.write(&format!("work/left/{name}"), name);
    }
    h.reread().focus("a.jpg");
    // Select the three pictures (Insert selects and moves down).
    h.keys("Insert*3 @multi_rename_tool");
    let Some(PopupType::MultiRename(dialog)) = h.state.dialogs.top() else {
        panic!("multi-rename dialog expected\n{}", h.screen_text());
    };
    assert_eq!(dialog.sources.len(), 3);
    // Name mask: "photo_" + 2-digit counter.
    h.keys("End Backspace*10").text("photo_[C]");
    h.keys("Down*9 End Backspace*3").text("2");
    h.assert_screen("photo_01.jpg");
    h.assert_screen("photo_03.jpg");
    h.keys("Down Enter");
    h.wait_until("renames", |h| h.exists("work/left/photo_03.jpg"));
    for (n, original) in ["a.jpg", "b.jpg", "c.jpg"].iter().enumerate() {
        let renamed = format!("work/left/photo_{:02}.jpg", n + 1);
        assert_eq!(h.read(&renamed), original.as_bytes());
    }
}
