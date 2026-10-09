//! The editor and viewer screens run the commands of the `[editor]` /
//! `[viewer]` keymap sections, and global panel actions keep working over
//! them with whatever keys the keymap gives them.

use crate::app::state::{PopupType, Screen};
use crate::keybindings::KeybindingResolver;
use crate::test_harness::Harness;

/// A harness with `overrides` (`id`, `keys`) applied and `notes.txt` listed.
fn with_overrides(overrides: &[(&str, &str)]) -> Harness {
    let mut h = Harness::new();
    for (id, keys) in overrides {
        h.ctx.config.keybindings.set_override("all", id, keys);
    }
    h.ctx.resolver = KeybindingResolver::new(&h.ctx.config);
    h.write("work/left/notes.txt", "");
    h.reread().focus("notes.txt");
    h
}

fn on_screen(h: &Harness) -> &'static str {
    match h.state.screens.get(h.state.active_screen_idx) {
        Some(Screen::Editor(_)) => "editor",
        Some(Screen::Viewer(_)) => "viewer",
        _ => "panels",
    }
}

#[test]
fn editor_saves_with_a_rebound_key() {
    let mut h = with_overrides(&[("editor.save", "Ctrl+w")]);
    h.keys("@edit");
    assert_eq!(on_screen(&h), "editor");
    h.text("hi").keys("Ctrl+w");
    h.settle();
    let saved = std::fs::read_to_string(h.left().join("notes.txt")).unwrap();
    assert_eq!(saved.trim_end(), "hi");
    h.keys("F2");
    assert!(
        h.state.dialogs.is_none(),
        "F2 no longer saves, nor opens anything"
    );
}

#[test]
fn global_actions_follow_their_keys_over_the_editor() {
    let mut h = with_overrides(&[("screens_list", "F11")]);
    h.keys("@edit F11");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ScreensMenu { .. })
    ));
}

#[test]
fn viewer_runs_rebound_commands() {
    let mut h = with_overrides(&[("viewer.quit", "q, Esc")]);
    h.keys("@view");
    assert_eq!(on_screen(&h), "viewer");
    h.keys("q");
    assert_eq!(on_screen(&h), "panels");
}
