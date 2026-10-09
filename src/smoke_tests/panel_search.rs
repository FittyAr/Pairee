//! In-panel search (`/`, `n`, `N`) and Norton Commander's Alt+letter
//! quick search.

use crate::app::state::PopupType;
use crate::keybindings::{Action, KeybindingResolver};
use crate::test_harness::Harness;

fn with_fruit() -> Harness {
    let mut h = Harness::new();
    for name in ["apple.txt", "banana.txt", "grape.txt", "pineapple.txt"] {
        h.write(&format!("work/left/{name}"), name);
    }
    h.reread().focus("..");
    h
}

#[test]
fn search_jumps_while_typing_and_n_repeats() {
    let mut h = with_fruit();
    h.dispatch(Action::FindInPanel);
    h.text("ap");
    assert_eq!(h.cursor_name(), "apple.txt");
    h.keys("Down");
    assert_eq!(h.cursor_name(), "grape.txt");
    h.keys("Enter");
    assert!(h.state.dialogs.is_none());
    h.dispatch(Action::FindNext);
    assert_eq!(h.cursor_name(), "pineapple.txt");
    h.dispatch(Action::FindNext);
    assert_eq!(h.cursor_name(), "apple.txt", "wraps around");
    h.dispatch(Action::FindPrev);
    assert_eq!(h.cursor_name(), "pineapple.txt");
}

#[test]
fn escape_returns_to_where_the_search_started() {
    let mut h = with_fruit();
    h.focus("banana.txt");
    h.dispatch(Action::FindInPanel);
    h.text("grape");
    assert_eq!(h.cursor_name(), "grape.txt");
    h.keys("Esc");
    assert_eq!(h.cursor_name(), "banana.txt");
    assert!(h.state.last_panel_search.is_none());
}

#[test]
fn alt_letter_starts_a_quick_search_when_the_preset_asks() {
    let mut h = with_fruit();
    h.write(
        "home/config/keymaps/far.toml",
        "extends = \"norton\"\n[options]\nalt_quick_search = true\n",
    );
    h.ctx.config.keybindings.preset = "far".into();
    h.ctx.resolver = KeybindingResolver::new(&h.ctx.config);
    h.keys("Alt+p");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::PanelSearch { .. })
    ));
    assert_eq!(h.cursor_name(), "pineapple.txt", "prefix match");
    h.text("i");
    assert_eq!(h.cursor_name(), "pineapple.txt");
    h.keys("Esc");
    assert_eq!(
        h.cursor_name(),
        "pineapple.txt",
        "quick search keeps the position"
    );
}
