//! Hotlist (Ctrl+\) and folder shortcuts (Ctrl+Alt+1…9), persisted in the
//! sandbox configuration directory.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

fn bookmarks_file(h: &Harness) -> String {
    std::fs::read_to_string(h.config_dir().join("bookmarks.toml")).unwrap_or_default()
}

fn hotlist_index(h: &Harness, needle: &str) -> Option<usize> {
    match h.state.dialogs.top() {
        Some(PopupType::Hotlist { entries, .. }) => entries
            .iter()
            .position(|e| e.path.to_string_lossy().contains(needle)),
        _ => panic!("hotlist expected\n{}", h.screen_text()),
    }
}

#[test]
fn hotlist_add_jump_remove() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        h.mkdir("work/left/proyecto");
        h.reread().focus("proyecto").keys("Enter @hotlist");
        h.keys("Insert");
        assert!(bookmarks_file(&h).contains("proyecto"), "{keymap}");
        let idx = hotlist_index(&h, "proyecto").expect("added");
        h.assert_screen("proyecto");
        h.keys("Esc Backspace");
        assert_eq!(h.state.get_active_panel().current_path, h.left());

        h.keys(&format!("@hotlist Home Down*{idx} Enter"));
        assert_eq!(
            h.state.get_active_panel().current_path,
            h.left().join("proyecto"),
            "{keymap}"
        );

        h.keys(&format!("@hotlist Home Down*{idx} Delete"));
        assert_eq!(hotlist_index(&h, "proyecto"), None);
        assert!(!bookmarks_file(&h).contains("proyecto"), "{keymap}");
        h.keys("Esc");
    }
}

#[test]
fn folder_shortcuts_assign_jump_and_persist() {
    let mut h = Harness::new();
    h.mkdir("work/left/destino");
    h.reread()
        .focus("destino")
        .keys("Enter @folder_shortcuts_config");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::FolderShortcuts { .. })
    ));
    h.keys("3 Esc");
    let target = h.left().join("destino");
    assert_eq!(h.state.folder_shortcuts.get(&3), Some(&target));
    assert!(bookmarks_file(&h).contains("destino"));

    h.keys("Backspace @go_folder_shortcut_3");
    assert_eq!(h.state.get_active_panel().current_path, target);

    // A new run loads the shortcut from the sandbox.
    let h = h.restart(|b| b);
    assert_eq!(h.state.folder_shortcuts.get(&3), Some(&target));
}
