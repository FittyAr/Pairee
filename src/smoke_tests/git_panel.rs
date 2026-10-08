//! Git panel: open on a sandbox repository (loaded in the background),
//! stage a file, commit, see the commit in the log.

use crate::app::state::{GitPanelState, PopupType};
use crate::test_harness::Harness;

fn panel(h: &Harness) -> Option<&GitPanelState> {
    h.state.dialogs.iter().find_map(|popup| match popup {
        PopupType::GitPanel(panel) => Some(panel),
        _ => None,
    })
}

fn staged(h: &Harness, name: &str) -> bool {
    panel(h).is_some_and(|p| {
        p.status_entries
            .iter()
            .any(|e| e.path == name && e.is_staged && !e.is_unstaged)
    })
}

#[test]
fn stage_commit_and_log() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder()
            .keymap(keymap)
            .settings(|s| {
                s.git_author_name = "Smoke Test".into();
                s.git_author_email = "smoke@example.invalid".into();
            })
            .build();
        let repo = git2::Repository::init(h.left()).unwrap();
        h.write("work/left/hola.txt", "hola");
        h.write("work/left/otro.txt", "otro");
        h.reread().keys("@open_git_panel");
        h.wait_until("git status", |h| {
            panel(h).is_some_and(|p| p.status_entries.len() == 2)
        });
        h.assert_screen("hola.txt");

        // Space stages the file under the cursor.
        let idx = panel(&h)
            .unwrap()
            .status_entries
            .iter()
            .position(|e| e.path == "hola.txt")
            .unwrap();
        h.keys(&format!("Home Down*{idx} Space"));
        h.wait_until("staged", |h| staged(h, "hola.txt"));
        assert!(!staged(&h, "otro.txt"), "{keymap}");

        // c: commit message prompt; only the staged file is committed.
        h.keys("c").text("primer commit").keys("Enter");
        h.wait_until("commit", |_| repo.head().is_ok());
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap().trim(), "primer commit");
        let tree = head.tree().unwrap();
        assert!(tree.get_name("hola.txt").is_some());
        assert!(tree.get_name("otro.txt").is_none(), "{keymap}");

        // Close the confirmation, then the log tab lists the commit.
        while !matches!(h.state.dialogs.top(), Some(PopupType::GitPanel(_))) {
            assert!(h.state.dialogs.is_some(), "git panel closed");
            h.keys("Esc");
        }
        h.keys("Tab");
        h.wait_until("git log", |h| {
            panel(h).is_some_and(|p| p.active_tab == 1 && !p.log_entries.is_empty())
        });
        h.assert_screen("primer commit");
        h.keys("Esc");
        assert!(h.state.dialogs.is_none());
    }
}
