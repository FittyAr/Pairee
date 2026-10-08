//! Multi-rename dialog: opening, key handling, execution and rendering.

use super::handle;
use crate::app::actions::fs_ops::multi_rename;
use crate::app::context::AppContext;
use crate::app::state::popup::MultiRenameState as Dialog;
use crate::app::state::{AppState, PopupType};
use crate::config::AppConfig;
use crate::config::localization::t;
use crate::fs::FileEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use std::path::{Path, PathBuf};

fn entry(dir: &Path, name: &str) -> FileEntry {
    FileEntry {
        name: name.into(),
        path: dir.join(name),
        size: 0,
        is_dir: false,
        is_symlink: false,
        modified: None,
    }
}

/// App whose active panel lists `names` inside `dir`, all selected.
fn app(dir: &Path, names: &[&str]) -> (AppContext, AppState) {
    let context = AppContext::new(AppConfig::default());
    let mut state = AppState::new(dir.to_path_buf(), dir.to_path_buf());
    let panel = state.get_active_panel_mut();
    panel.current_path = dir.to_path_buf();
    panel.entries = std::iter::once("..")
        .chain(names.iter().copied())
        .map(|name| entry(dir, name))
        .collect();
    (context, state)
}

fn select(state: &mut AppState, names: &[&str]) {
    let panel = state.get_active_panel_mut();
    let dir = panel.current_path.clone();
    for name in names {
        panel.selected_paths.insert(dir.join(name));
    }
}

fn dialog(state: &AppState) -> &Dialog {
    match state.dialogs.top() {
        Some(PopupType::MultiRename(dialog)) => dialog,
        other => panic!("multi-rename dialog expected, got {other:?}"),
    }
}

fn new_names(state: &AppState) -> Vec<String> {
    dialog(state)
        .preview
        .rows
        .iter()
        .map(|r| r.new.clone())
        .collect()
}

fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
    let key = KeyEvent::new(code, KeyModifiers::NONE);
    assert_eq!(handle(state, key, context), Ok(None));
}

fn type_text(state: &mut AppState, context: &mut AppContext, text: &str) {
    for c in text.chars() {
        press(state, context, KeyCode::Char(c));
    }
}

#[test]
fn opens_with_the_selection_in_listing_order() {
    let dir = PathBuf::from("work");
    let (_, mut state) = app(&dir, &["b.txt", "a.txt", "c.txt"]);
    select(&mut state, &["c.txt", "b.txt"]);
    assert!(multi_rename::handle(&mut state));
    let d = dialog(&state);
    let names: Vec<String> = d.sources.iter().map(|s| s.name()).collect();
    assert_eq!(names, ["b.txt", "c.txt"]);
    assert_eq!(d.siblings, ["b.txt", "a.txt", "c.txt"]);
    assert_eq!(
        new_names(&state),
        ["b.txt", "c.txt"],
        "default masks keep names"
    );
}

#[test]
fn without_selection_the_cursor_entry_is_used_and_dotdot_is_refused() {
    let dir = PathBuf::from("work");
    let (_, mut state) = app(&dir, &["a.txt"]);
    multi_rename::handle(&mut state);
    assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
    state.dialogs.clear();
    state.get_active_panel_mut().cursor_index = 1;
    multi_rename::handle(&mut state);
    assert_eq!(new_names(&state), ["a.txt"]);
}

#[test]
fn typing_updates_the_preview_live() {
    let dir = PathBuf::from("work");
    let (mut context, mut state) = app(&dir, &["a.txt", "b.txt"]);
    select(&mut state, &["a.txt", "b.txt"]);
    multi_rename::handle(&mut state);
    type_text(&mut state, &mut context, "_[C]");
    assert_eq!(new_names(&state), ["a_1.txt", "b_2.txt"]);
    // Extension mask (next row): replace with "bak".
    press(&mut state, &mut context, KeyCode::Tab);
    for _ in 0..3 {
        press(&mut state, &mut context, KeyCode::Backspace);
    }
    type_text(&mut state, &mut context, "bak");
    assert_eq!(new_names(&state), ["a_1.bak", "b_2.bak"]);
    // Counter digits (row 9): 3.
    while dialog(&state).focus != 9 {
        press(&mut state, &mut context, KeyCode::Tab);
    }
    press(&mut state, &mut context, KeyCode::Backspace);
    type_text(&mut state, &mut context, "3");
    assert_eq!(new_names(&state), ["a_001.bak", "b_002.bak"]);
}

#[test]
fn regex_toggle_case_selector_and_errors() {
    let dir = PathBuf::from("work");
    let (mut context, mut state) = app(&dir, &["img_01.jpg"]);
    state.get_active_panel_mut().cursor_index = 1;
    multi_rename::handle(&mut state);
    for _ in 0..2 {
        press(&mut state, &mut context, KeyCode::Tab);
    }
    type_text(&mut state, &mut context, "(\\d+)");
    press(&mut state, &mut context, KeyCode::Tab);
    type_text(&mut state, &mut context, "n$1");
    assert_eq!(
        new_names(&state),
        ["img_01.jpg"],
        "literal search: no match"
    );
    press(&mut state, &mut context, KeyCode::Tab);
    press(&mut state, &mut context, KeyCode::Char(' '));
    assert!(dialog(&state).regex);
    assert_eq!(new_names(&state), ["img_n01.jpg"]);
    // Case selector: Right cycles forward (unchanged -> lower -> UPPER).
    press(&mut state, &mut context, KeyCode::Down);
    press(&mut state, &mut context, KeyCode::Down);
    press(&mut state, &mut context, KeyCode::Right);
    press(&mut state, &mut context, KeyCode::Right);
    assert_eq!(new_names(&state), ["IMG_N01.JPG"]);
    // Broken pattern: error shown, Rename refused.
    for _ in 0..4 {
        press(&mut state, &mut context, KeyCode::Up);
    }
    type_text(&mut state, &mut context, "(");
    assert!(dialog(&state).error.is_some());
    assert!(!dialog(&state).can_run());
}

#[test]
fn conflicts_block_enter_and_esc_closes() {
    let dir = PathBuf::from("work");
    let (mut context, mut state) = app(&dir, &["a.txt", "b.txt"]);
    select(&mut state, &["a.txt", "b.txt"]);
    multi_rename::handle(&mut state);
    for _ in 0..3 {
        press(&mut state, &mut context, KeyCode::Backspace);
    }
    type_text(&mut state, &mut context, "same");
    assert_eq!(dialog(&state).preview.conflicts(), 2);
    press(&mut state, &mut context, KeyCode::Enter);
    assert!(!dialog(&state).running, "nothing starts with conflicts");
    press(&mut state, &mut context, KeyCode::PageDown);
    press(&mut state, &mut context, KeyCode::Esc);
    assert!(state.dialogs.top().is_none());
}

#[test]
fn enter_renames_files_and_closes_the_dialog() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    for name in ["1.txt", "2.txt"] {
        std::fs::write(dir.join(name), name).unwrap();
    }
    let (mut context, mut state) = app(dir, &["1.txt", "2.txt"]);
    select(&mut state, &["1.txt", "2.txt"]);
    multi_rename::handle(&mut state);
    // Swap the names: [C] from 2, step -1.
    for _ in 0..3 {
        press(&mut state, &mut context, KeyCode::Backspace);
    }
    type_text(&mut state, &mut context, "[C]");
    while dialog(&state).focus != 7 {
        press(&mut state, &mut context, KeyCode::Tab);
    }
    press(&mut state, &mut context, KeyCode::Backspace);
    type_text(&mut state, &mut context, "2");
    press(&mut state, &mut context, KeyCode::Tab);
    press(&mut state, &mut context, KeyCode::Backspace);
    type_text(&mut state, &mut context, "-1");
    assert_eq!(new_names(&state), ["2.txt", "1.txt"]);
    press(&mut state, &mut context, KeyCode::Enter);
    // No Tokio runtime in unit tests: the job ran inline.
    multi_rename::poll(&mut state, &context);
    assert!(state.dialogs.top().is_none(), "{:?}", state.dialogs.top());
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    assert_eq!(
        (read("1.txt"), read("2.txt")),
        ("2.txt".into(), "1.txt".into())
    );
}

fn render(context: &AppContext, state: &AppState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|f| crate::ui::draw_ui(f, context, state))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn renders_fields_preview_and_conflicts() {
    let dir = PathBuf::from("work");
    let (mut context, mut state) = app(&dir, &["alpha.txt", "beta.txt"]);
    select(&mut state, &["alpha.txt", "beta.txt"]);
    multi_rename::handle(&mut state);
    type_text(&mut state, &mut context, "-new");
    let screen = render(&context, &state);
    for expected in [
        t("multi_rename_title").trim().to_string(),
        t("multi_rename_name_mask"),
        t("multi_rename_col_new"),
        "alpha-new.txt".into(),
        "beta-new.txt".into(),
        t("multi_rename_ok"),
    ] {
        assert!(screen.contains(&expected), "missing {expected:?}");
    }
    // Both names collide: the status column says so.
    for _ in 0.."[N]-new".len() {
        press(&mut state, &mut context, KeyCode::Backspace);
    }
    type_text(&mut state, &mut context, "x");
    let screen = render(&context, &state);
    assert!(screen.contains(&t("multi_rename_issue_duplicate")));
    // Tiny terminals do not panic.
    let mut tiny = Terminal::new(TestBackend::new(20, 6)).unwrap();
    tiny.draw(|f| crate::ui::draw_ui(f, &context, &state))
        .unwrap();
}

#[test]
fn shift_f6_opens_multi_rename_in_every_keymap() {
    for preset in ["norton", "neovim", "vscode"] {
        let mut config = AppConfig::default();
        config.keybindings.preset = preset.into();
        let mut resolver = crate::keybindings::KeybindingResolver::new(&config);
        let key = KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT);
        assert_eq!(
            resolver.resolve(key),
            Some(crate::keybindings::Action::MultiRename),
            "{preset}"
        );
    }
}

#[test]
fn long_preview_registers_a_wheel_scrollable_scrollbar() {
    use crate::ui::scrollbar::ScrollTargetId;
    let dir = PathBuf::from("work");
    let names: Vec<String> = (0..80).map(|i| format!("f{i:02}.txt")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let (context, mut state) = app(&dir, &names);
    select(&mut state, &names);
    multi_rename::handle(&mut state);
    state.scrollbar.clear_targets();
    render(&context, &state);
    let target = state
        .scrollbar
        .targets_snapshot()
        .into_iter()
        .find(|t| t.id == ScrollTargetId::MultiRenamePreview)
        .expect("preview scrollbar registered");
    assert_eq!(target.content_len, 80);
    assert!(target.wheel_area.width > target.area.width);

    let wheel = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column: target.wheel_area.x + 2,
        row: target.wheel_area.y + 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(crate::app::app::scrollbar_mouse::handle_scrollbar_mouse(
        &mut state, wheel
    ));
    assert!(dialog(&state).scroll > 0);
}
