use super::*;
use crate::app::state::popup::TransferPromptOp;
use crate::config::AppConfig;
use crossterm::event::KeyModifiers;
use std::path::PathBuf;

fn context() -> AppContext {
    AppContext::new(AppConfig {
        settings: crate::config::settings::Settings::default(),
        theme: crate::config::theme::Theme::default(),
        keybindings: crate::config::keybindings::KeybindingsConfig::default(),
    })
}

fn open(op: TransferPromptOp) -> AppState {
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    state.dialogs.replace(PopupType::TransferPrompt(Prompt::new(
        op,
        vec![PathBuf::from("a"), PathBuf::from("b")],
        PathBuf::from("dst"),
    )));
    state
}

fn press(state: &mut AppState, code: KeyCode) -> Result<Option<Action>, ()> {
    handle(
        state,
        KeyEvent::new(code, KeyModifiers::NONE),
        &mut context(),
    )
}

fn prompt(state: &AppState) -> &Prompt {
    match state.dialogs.top() {
        Some(PopupType::TransferPrompt(p)) => p,
        other => panic!("expected transfer prompt, got {other:?}"),
    }
}

#[test]
fn rows_wrap_and_buttons_cycle() {
    let mut state = open(TransferPromptOp::Move);
    press(&mut state, KeyCode::Up).unwrap();
    assert_eq!(prompt(&state).cursor_idx, Prompt::BUTTON_CANCEL);
    press(&mut state, KeyCode::Right).unwrap();
    assert_eq!(prompt(&state).cursor_idx, Prompt::BUTTON_SUBMIT);
    press(&mut state, KeyCode::Left).unwrap();
    assert_eq!(prompt(&state).cursor_idx, Prompt::BUTTON_CANCEL);
    press(&mut state, KeyCode::Tab).unwrap();
    assert_eq!(prompt(&state).cursor_idx, Prompt::ROW_INPUT);
}

#[test]
fn input_row_edits_text_and_other_rows_toggle() {
    let mut state = open(TransferPromptOp::Copy);
    press(&mut state, KeyCode::Char('/')).unwrap();
    press(&mut state, KeyCode::Char('x')).unwrap();
    press(&mut state, KeyCode::Left).unwrap();
    press(&mut state, KeyCode::Backspace).unwrap();
    assert_eq!(prompt(&state).input.text(), "dstx");
    press(&mut state, KeyCode::Down).unwrap();
    press(&mut state, KeyCode::Char(' ')).unwrap();
    assert_eq!(prompt(&state).already_existing, 1);
    press(&mut state, KeyCode::Char('q')).unwrap();
    assert_eq!(prompt(&state).input.text(), "dstx");
}

#[test]
fn filter_button_opens_filter_over_the_dialog() {
    let mut state = open(TransferPromptOp::Copy);
    if let Some(PopupType::TransferPrompt(p)) = state.dialogs.top_mut() {
        p.cursor_idx = Prompt::BUTTON_FILTER;
        p.filter_mask = "*.txt".into();
    }
    press(&mut state, KeyCode::Enter).unwrap();
    match state.dialogs.top() {
        Some(PopupType::CopyMoveFilterPrompt { input, previous }) => {
            assert_eq!(input, "*.txt");
            assert!(matches!(**previous, PopupType::TransferPrompt(_)));
        }
        other => panic!("expected filter prompt, got {other:?}"),
    }
}

#[test]
fn cancel_and_escape_close() {
    let mut state = open(TransferPromptOp::Copy);
    press(&mut state, KeyCode::Esc).unwrap();
    assert!(state.dialogs.is_empty());
}
