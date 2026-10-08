use super::*;
use crate::app::editor::EditorClipboard;
use crate::app::state::Screen;
use crate::config::AppConfig;
use std::path::PathBuf;

fn setup() -> (AppState, AppContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    std::fs::write(&path, "").unwrap();
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    let context = AppContext::new(AppConfig::default());
    crate::app::editor::open::open_in_editor(&mut state, path, &context.config.settings);
    state.editor_clipboard = EditorClipboard::internal_only();
    (state, context, dir)
}

fn press_mod(state: &mut AppState, context: &mut AppContext, code: KeyCode, m: KeyModifiers) {
    handle_editor_screen(state, KeyEvent::new(code, m), context).expect("editor handles key");
}

fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
    press_mod(state, context, code, KeyModifiers::NONE);
}

fn editor(state: &AppState) -> &EditorState {
    match state.screens.get(state.active_screen_idx) {
        Some(Screen::Editor(ed)) => ed,
        _ => panic!("expected editor screen"),
    }
}

#[test]
fn typing_and_editing_non_ascii_text() {
    let (mut state, mut context, _dir) = setup();
    for c in "ñandú 👍🏽e\u{301}".chars() {
        press(&mut state, &mut context, KeyCode::Char(c));
    }
    assert_eq!(editor(&state).lines[0], "ñandú 👍🏽e\u{301}");

    press(&mut state, &mut context, KeyCode::Backspace); // é (combined)
    press(&mut state, &mut context, KeyCode::Left); // before 👍🏽
    press(&mut state, &mut context, KeyCode::Delete); // 👍🏽
    assert_eq!(editor(&state).lines[0], "ñandú ");

    press(&mut state, &mut context, KeyCode::Left); // before ' '
    press(&mut state, &mut context, KeyCode::Left); // before 'ú'
    press(&mut state, &mut context, KeyCode::Char('X'));
    press(&mut state, &mut context, KeyCode::Enter);
    let ed = editor(&state);
    assert_eq!(ed.lines, vec!["ñandX".to_string(), "ú ".to_string()]);

    press(&mut state, &mut context, KeyCode::Right); // after 'ú'
    press(&mut state, &mut context, KeyCode::Up); // byte col 2 snaps inside "ñandX"
    let ed = editor(&state);
    assert_eq!(ed.cursor_y, 0);
    assert!(ed.lines[0].is_char_boundary(ed.cursor_x));
    press(&mut state, &mut context, KeyCode::Backspace);
    assert_eq!(editor(&state).lines[0], "andX");
}

#[test]
fn ctrl_z_and_ctrl_y_undo_and_redo() {
    let (mut state, mut context, _dir) = setup();
    for c in "hi".chars() {
        press(&mut state, &mut context, KeyCode::Char(c));
    }
    press_mod(
        &mut state,
        &mut context,
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    );
    assert_eq!(editor(&state).lines, vec![String::new()]);
    assert!(!editor(&state).is_dirty());
    press_mod(
        &mut state,
        &mut context,
        KeyCode::Char('y'),
        KeyModifiers::CONTROL,
    );
    assert_eq!(editor(&state).lines, vec!["hi".to_string()]);
}

#[test]
fn f2_saves_and_shift_f2_opens_save_as() {
    let (mut state, mut context, _dir) = setup();
    press(&mut state, &mut context, KeyCode::Char('a'));
    press(&mut state, &mut context, KeyCode::F(2));
    let path = editor(&state).path.clone();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "a");
    assert!(!editor(&state).is_dirty());
    press_mod(&mut state, &mut context, KeyCode::F(2), KeyModifiers::SHIFT);
    assert!(matches!(
        state.dialogs.top(),
        Some(PopupType::EditorSaveAsPrompt { .. })
    ));
}

fn ctrl(state: &mut AppState, context: &mut AppContext, c: char) {
    press_mod(state, context, KeyCode::Char(c), KeyModifiers::CONTROL);
}

fn type_text(state: &mut AppState, context: &mut AppContext, text: &str) {
    for c in text.chars() {
        press(state, context, KeyCode::Char(c));
    }
}

#[test]
fn shift_arrows_select_and_typing_replaces() {
    let (mut state, mut context, _dir) = setup();
    type_text(&mut state, &mut context, "abc日d");
    press(&mut state, &mut context, KeyCode::Home);
    press(&mut state, &mut context, KeyCode::Right);
    for _ in 0..3 {
        press_mod(
            &mut state,
            &mut context,
            KeyCode::Right,
            KeyModifiers::SHIFT,
        );
    }
    assert_eq!(editor(&state).selected_text().as_deref(), Some("bc日"));
    press(&mut state, &mut context, KeyCode::Char('X'));
    assert_eq!(editor(&state).lines, vec!["aXd".to_string()]);
    ctrl(&mut state, &mut context, 'z');
    assert_eq!(editor(&state).lines, vec!["abc日d".to_string()]);
}

#[test]
fn copy_cut_paste_round_trip() {
    let (mut state, mut context, _dir) = setup();
    type_text(&mut state, &mut context, "one two");
    press_mod(&mut state, &mut context, KeyCode::Home, KeyModifiers::SHIFT);
    ctrl(&mut state, &mut context, 'c');
    assert_eq!(editor(&state).lines, vec!["one two".to_string()]);
    press(&mut state, &mut context, KeyCode::End);
    ctrl(&mut state, &mut context, 'v');
    assert_eq!(editor(&state).lines, vec!["one twoone two".to_string()]);

    ctrl(&mut state, &mut context, 'a');
    ctrl(&mut state, &mut context, 'x');
    assert_eq!(editor(&state).lines, vec![String::new()]);
    ctrl(&mut state, &mut context, 'v');
    ctrl(&mut state, &mut context, 'v');
    assert_eq!(
        editor(&state).lines,
        vec!["one twoone twoone twoone two".to_string()]
    );
    ctrl(&mut state, &mut context, 'z');
    ctrl(&mut state, &mut context, 'z');
    assert_eq!(
        editor(&state).lines,
        vec![String::new()],
        "one step per paste"
    );
    ctrl(&mut state, &mut context, 'z');
    assert_eq!(editor(&state).lines, vec!["one twoone two".to_string()]);
}

/// Types two lines, then cuts the block "ab"/"ef" selected with `mods`
/// (after `prepare`).
fn cut_block_with(
    prepare: impl FnOnce(&mut AppState, &mut AppContext),
    mods: KeyModifiers,
) -> (AppState, AppContext, tempfile::TempDir) {
    let (mut state, mut context, dir) = setup();
    type_text(&mut state, &mut context, "abcd");
    press(&mut state, &mut context, KeyCode::Enter);
    type_text(&mut state, &mut context, "efgh");
    press(&mut state, &mut context, KeyCode::Home);
    prepare(&mut state, &mut context);
    press_mod(&mut state, &mut context, KeyCode::Right, mods);
    press_mod(&mut state, &mut context, KeyCode::Right, mods);
    press_mod(&mut state, &mut context, KeyCode::Up, mods);
    ctrl(&mut state, &mut context, 'x');
    (state, context, dir)
}

#[test]
fn alt_shift_arrows_cut_a_block() {
    let block = KeyModifiers::ALT | KeyModifiers::SHIFT;
    let (mut state, mut context, _dir) = cut_block_with(|_, _| {}, block);
    assert_eq!(
        editor(&state).lines,
        vec!["cd".to_string(), "gh".to_string()]
    );
    ctrl(&mut state, &mut context, 'v');
    assert_eq!(
        editor(&state).lines,
        vec!["ab".to_string(), "efcd".to_string(), "gh".to_string()],
        "a block pastes as text"
    );
}

#[test]
fn ctrl_alt_shift_arrows_select_a_block() {
    let mods = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT;
    let (state, _, _dir) = cut_block_with(|_, _| {}, mods);
    assert_eq!(editor(&state).lines, ["cd", "gh"]);
}

#[test]
fn block_mode_turns_shift_arrows_into_block_selection() {
    let toggle = |state: &mut AppState, context: &mut AppContext| ctrl(state, context, 'b');
    let (state, _, _dir) = cut_block_with(toggle, KeyModifiers::SHIFT);
    assert_eq!(editor(&state).lines, ["cd", "gh"]);
    assert!(editor(&state).block_mode);
    // Toggled twice: back to a stream selection ("cd" and the line break).
    let twice = |state: &mut AppState, context: &mut AppContext| {
        ctrl(state, context, 'b');
        ctrl(state, context, 'b');
    };
    let (state, _, _dir) = cut_block_with(twice, KeyModifiers::SHIFT);
    assert_eq!(editor(&state).lines, ["abefgh"]);
}

#[test]
fn bracketed_paste_inserts_all_lines() {
    let (mut state, mut context, _dir) = setup();
    type_text(&mut state, &mut context, "[]");
    press(&mut state, &mut context, KeyCode::Left);
    crate::app::input::handle_paste(&mut state, "x\r\ny");
    assert_eq!(
        editor(&state).lines,
        vec!["[x".to_string(), "y]".to_string()]
    );
    assert!(state.cli_input.is_empty());
}

#[test]
fn escape_clears_selection_before_closing() {
    let (mut state, mut context, _dir) = setup();
    type_text(&mut state, &mut context, "ab");
    press_mod(&mut state, &mut context, KeyCode::Left, KeyModifiers::SHIFT);
    press(&mut state, &mut context, KeyCode::Esc);
    assert!(!editor(&state).has_selection());
    assert!(matches!(
        state.screens.get(state.active_screen_idx),
        Some(Screen::Editor(_))
    ));
}

#[test]
fn mouse_drag_selects() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (mut state, mut context, _dir) = setup();
    type_text(&mut state, &mut context, "hello");
    editor(&state)
        .viewport
        .set(crate::app::editor::viewport::Viewport {
            x: 1,
            y: 1,
            width: 20,
            height: 5,
            scroll_x: 0,
        });
    let event = |kind, column| MouseEvent {
        kind,
        column,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(handle_editor_mouse(
        &mut state,
        event(MouseEventKind::Down(MouseButton::Left), 2)
    ));
    assert!(handle_editor_mouse(
        &mut state,
        event(MouseEventKind::Drag(MouseButton::Left), 5)
    ));
    assert_eq!(editor(&state).selected_text().as_deref(), Some("ell"));
}
