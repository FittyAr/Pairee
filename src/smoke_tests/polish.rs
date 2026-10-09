//! G.5 polish: Git panel local operations as background jobs, keymap
//! chords that used to be rejected, keymap-driven F-key labels and the
//! editor's block mode.

use crate::app::state::{GitPanelState, PopupType, Screen};
use crate::config::localization::t;
use crate::test_harness::Harness;

fn panel(h: &Harness) -> Option<&GitPanelState> {
    h.state.dialogs.iter().find_map(|popup| match popup {
        PopupType::GitPanel(panel) => Some(panel),
        _ => None,
    })
}

#[test]
fn git_discard_and_stash_run_as_jobs() {
    let mut h = Harness::new();
    let repo = git2::Repository::init(h.left()).unwrap();
    h.write("work/left/a.txt", "one");
    {
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("a.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("T", "t@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "c1", &tree, &[])
            .unwrap();
    }
    h.write("work/left/a.txt", "changed");
    h.reread().keys("@open_git_panel");
    h.wait_until("status", |h| {
        panel(h).is_some_and(|p| p.status_entries.len() == 1)
    });

    // x + Enter: discard, through the confirmation and a background job.
    // Keys settle until the job is done, so its result is already applied.
    h.keys("Home x Enter");
    assert!(!h.state.git_panel.local.is_running());
    assert_eq!(h.read("work/left/a.txt"), b"one");
    h.wait_until("clean status", |h| {
        panel(h).is_some_and(|p| p.status_entries.is_empty())
    });
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::GitPanel(_))
    ));

    // s + Enter: stash save; the Stash tab lists it.
    h.write("work/left/a.txt", "again");
    h.keys("r");
    h.wait_until("modified", |h| {
        panel(h).is_some_and(|p| p.status_entries.len() == 1)
    });
    h.keys("s").text("wip").keys("Enter");
    h.wait_until("stashed", |h| {
        panel(h).is_some_and(|p| p.stash_entries.len() == 1)
    });
    assert_eq!(h.read("work/left/a.txt"), b"one");
}

#[test]
fn fixed_keymap_chords_reach_their_actions() {
    for keymap in ["norton", "neovim", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        assert!(h.ctx.resolver.load_report().ok(), "{keymap}");
        // Ctrl+Shift+P as a terminal with Shift reporting sends it.
        h.keys("Ctrl+Shift+P");
        assert!(
            matches!(
                h.state.dialogs.top(),
                Some(PopupType::CommandPalette { .. })
            ),
            "{keymap}: {}",
            h.screen_text()
        );
        h.keys("Esc Ctrl+Shift+K");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::WhichKey { .. })),
            "{keymap}"
        );
    }
}

#[test]
fn fkey_bar_shift_row_comes_from_the_keymap() {
    // Only the norton preset shows the F-key bar; wide enough for whole
    // six-letter labels.
    let mut h = Harness::builder().keymap("norton").size(200, 30).build();
    // Normal -> Ctrl -> Alt -> Shift.
    h.keys("@cycle_fkeys_modifiers*3").render();
    h.assert_screen(&t("fkey_sh_mrename"));
    h.assert_screen(&t("fkey_sh_pack"));
    // A fourth press goes back to following the held modifiers.
    h.keys("@cycle_fkeys_modifiers");
    assert_eq!(h.state.fkeys_modifier_override, None);
}

#[test]
fn editor_block_mode_selects_columns_with_shift_arrows() {
    let mut h = Harness::new();
    h.write("work/left/cols.txt", "abcd\nefgh\n");
    h.reread().focus("cols.txt").keys("@edit");
    h.keys("Ctrl+b").render();
    h.assert_screen(&t("editor_block_mode_flag"));
    h.keys("Down Home Shift+Right*2 Shift+Up Ctrl+x");
    match h.state.screens.get(h.state.active_screen_idx) {
        Some(Screen::Editor(ed)) => assert_eq!(ed.lines[..2], ["cd", "gh"]),
        _ => panic!("editor expected\n{}", h.screen_text()),
    }
}
