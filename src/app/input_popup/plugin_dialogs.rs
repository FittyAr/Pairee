use crate::app::context::AppContext;
use crate::app::state::{AppState, PendingPluginReply, PopupType};
use crate::keybindings::Action;
use crate::plugin::manager::InputDialogResult;
use crate::plugin::manager::dialogs::key_matches_spec;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top() {
        Some(PopupType::PluginConfirm { .. }) => handle_confirm(state, key),
        Some(PopupType::PluginInput { .. }) => handle_input(state, key),
        Some(PopupType::PluginWhich { .. }) => handle_which(state, key),
        _ => Err(()),
    }
}

fn handle_confirm(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    let Some(PopupType::PluginConfirm { cursor_idx, .. }) = state.dialogs.top().cloned() else {
        return Err(());
    };
    match key.code {
        KeyCode::Left | KeyCode::BackTab => {
            set_confirm_cursor(state, 0);
            Ok(None)
        }
        KeyCode::Right | KeyCode::Tab => {
            set_confirm_cursor(state, 1);
            Ok(None)
        }
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('s') | KeyCode::Char('S') => {
            finish_confirm(state, true);
            Ok(None)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            finish_confirm(state, false);
            Ok(None)
        }
        KeyCode::Enter => {
            finish_confirm(state, cursor_idx == 0);
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn set_confirm_cursor(state: &mut AppState, cursor_idx: usize) {
    if let Some(PopupType::PluginConfirm {
        cursor_idx: idx, ..
    }) = state.dialogs.top_mut()
    {
        *idx = cursor_idx;
        state.mark_ui_dirty();
    }
}

fn finish_confirm(state: &mut AppState, accepted: bool) {
    if let Some(PendingPluginReply::Confirm(tx)) = state.plugins.pending_dialog.take() {
        let _ = tx.send(accepted);
    } else {
        state.plugins.cancel_pending();
    }
    state.dialogs.clear();
    state.mark_ui_dirty();
}

fn handle_input(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    match key.code {
        KeyCode::Char(c) => {
            if let Some(PopupType::PluginInput { input, .. }) = state.dialogs.top_mut() {
                input.push(c);
                state.mark_ui_dirty();
            }
            Ok(None)
        }
        KeyCode::Backspace => {
            if let Some(PopupType::PluginInput { input, .. }) = state.dialogs.top_mut() {
                input.pop();
                state.mark_ui_dirty();
            }
            Ok(None)
        }
        KeyCode::Enter => {
            let value = match state.dialogs.top() {
                Some(PopupType::PluginInput { input, .. }) => input.clone(),
                _ => String::new(),
            };
            finish_input(state, value, 1);
            Ok(None)
        }
        KeyCode::Esc => {
            finish_input(state, String::new(), 2);
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn finish_input(state: &mut AppState, value: String, event: i32) {
    match state.plugins.pending_dialog.take() {
        Some(PendingPluginReply::Input(tx)) => {
            let _ = tx.send(InputDialogResult { value, event });
        }
        Some(PendingPluginReply::LegacyInput(tx)) => {
            let _ = tx.send(value);
        }
        other => {
            state.plugins.pending_dialog = other;
            state.plugins.cancel_pending();
        }
    }
    state.dialogs.clear();
    state.mark_ui_dirty();
}

fn handle_which(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    if matches!(key.code, KeyCode::Esc) {
        finish_which(state, None);
        return Ok(None);
    }

    let pressed = crate::keybindings::resolver::key_event_to_string(key);
    if pressed.is_empty() {
        return Ok(None);
    }

    let Some(PopupType::PluginWhich { candidates, .. }) = state.dialogs.top() else {
        return Err(());
    };
    let matched = candidates.iter().enumerate().find_map(|(i, cand)| {
        cand.on
            .iter()
            .any(|spec| key_matches_spec(&pressed, spec))
            .then_some(i + 1)
    });
    if let Some(idx) = matched {
        finish_which(state, Some(idx));
    }
    Ok(None)
}

fn finish_which(state: &mut AppState, index: Option<usize>) {
    if let Some(PendingPluginReply::Which(tx)) = state.plugins.pending_dialog.take() {
        let _ = tx.send(index);
    } else {
        state.plugins.cancel_pending();
    }
    state.dialogs.clear();
    state.mark_ui_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::plugin::manager::{
        WhichCandidate, dialogs::open_confirm, dialogs::open_input, dialogs::open_which,
    };
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
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
        assert_eq!(rx.blocking_recv().unwrap(), true);
        assert!(state.dialogs.is_none());
    }

    #[test]
    fn confirm_esc_sends_false() {
        let mut state = state();
        let mut context = ctx();
        let (tx, rx) = oneshot::channel();
        open_confirm(&mut state, "t".into(), "m".into(), None, tx);
        handle(&mut state, make_key(KeyCode::Esc), &mut context).unwrap();
        assert_eq!(rx.blocking_recv().unwrap(), false);
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
        assert_eq!(rx.blocking_recv().unwrap(), false);
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
}
