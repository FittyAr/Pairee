use super::*;
use crate::config::AppConfig;
use crate::plugin::manager::{
    WhichCandidate, dialogs::open_confirm, dialogs::open_input, dialogs::open_which,
};
use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};
use std::path::PathBuf;
use tokio::sync::oneshot;

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn ctx() -> AppContext {
    AppContext::new(AppConfig {
        settings: crate::config::settings::Settings::default(),
        theme: crate::config::theme::Theme::default(),
        keybindings: crate::config::keybindings::KeybindingsConfig::default(),
    })
}

fn state() -> AppState {
    AppState::new(PathBuf::from("."), PathBuf::from("."))
}

#[test]
fn confirm_enter_on_yes_sends_true() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_confirm(&mut state, "t".into(), "m".into(), None, tx);
    let res = handle(&mut state, make_key(KeyCode::Enter), &mut context);
    assert!(res.is_ok());
    assert!(rx.blocking_recv().unwrap());
    assert!(state.dialogs.is_none());
}

#[test]
fn confirm_esc_sends_false() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_confirm(&mut state, "t".into(), "m".into(), None, tx);
    handle(&mut state, make_key(KeyCode::Esc), &mut context).unwrap();
    assert!(!rx.blocking_recv().unwrap());
    assert!(state.dialogs.is_none());
}

#[test]
fn confirm_tab_then_enter_sends_false() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_confirm(&mut state, "t".into(), "m".into(), None, tx);
    handle(&mut state, make_key(KeyCode::Tab), &mut context).unwrap();
    handle(&mut state, make_key(KeyCode::Enter), &mut context).unwrap();
    assert!(!rx.blocking_recv().unwrap());
}

#[test]
fn input_types_and_submits() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_input(
        &mut state,
        "Name".into(),
        "ab".into(),
        false,
        None,
        PendingPluginReply::Input(tx),
    );
    handle(&mut state, make_key(KeyCode::Char('c')), &mut context).unwrap();
    handle(&mut state, make_key(KeyCode::Enter), &mut context).unwrap();
    let result = rx.blocking_recv().unwrap();
    assert_eq!(result.value, "abc");
    assert_eq!(result.event, 1);
}

#[test]
fn input_esc_cancels_with_empty_value() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_input(
        &mut state,
        "Name".into(),
        "keep".into(),
        false,
        None,
        PendingPluginReply::Input(tx),
    );
    handle(&mut state, make_key(KeyCode::Esc), &mut context).unwrap();
    let result = rx.blocking_recv().unwrap();
    assert_eq!(result.value, "");
    assert_eq!(result.event, 2);
}

#[test]
fn which_matching_key_returns_one_based_index() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_which(
        &mut state,
        vec![
            WhichCandidate {
                on: vec!["a".into()],
                desc: Some("first".into()),
            },
            WhichCandidate {
                on: vec!["<C-c>".into(), "b".into()],
                desc: None,
            },
        ],
        false,
        tx,
    );
    handle(&mut state, make_key(KeyCode::Char('b')), &mut context).unwrap();
    assert_eq!(rx.blocking_recv().unwrap(), Some(2));
    assert!(state.dialogs.is_none());
}

#[test]
fn which_esc_returns_none() {
    let mut state = state();
    let mut context = ctx();
    let (tx, rx) = oneshot::channel();
    open_which(
        &mut state,
        vec![WhichCandidate {
            on: vec!["x".into()],
            desc: None,
        }],
        true,
        tx,
    );
    handle(&mut state, make_key(KeyCode::Esc), &mut context).unwrap();
    assert_eq!(rx.blocking_recv().unwrap(), None);
}
