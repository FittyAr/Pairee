//! Plugin dialog input handlers (M1).
//!
//! Handles key events for the three plugin-spawned popups:
//! - `PluginInputDialog` — text input, optional `obscure` mode.
//! - `PluginConfirmDialog` — yes/no with cursor toggle.
//! - `PluginWhichPrompt` — listen for a candidate key and return the
//!   1-based index of the matching candidate (or `None` on Esc).
//!
//! All three handlers take the `oneshot::Sender` from the popup
//! variant, send the result, and clear `active_popup` in a single
//! step so the awaiting plugin worker wakes up immediately.

use crate::app::state::{AppState, PopupType};
use crate::keybindings::resolver::key_event_to_string;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Top-level entry point. Returns `Ok(None)` when a popup was handled
/// (so the input event is fully consumed), or `Err(())` when no
/// plugin popup is active.
pub fn handle(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    match state.active_popup {
        Some(PopupType::PluginInputDialog { .. }) => handle_input(state, key),
        Some(PopupType::PluginConfirmDialog { .. }) => handle_confirm(state, key),
        Some(PopupType::PluginWhichPrompt { .. }) => handle_which(state, key),
        _ => Err(()),
    }
}

fn handle_input(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    // `take()` the popup so we own it and can rebuild a new variant
    // without needing `Clone` on the `oneshot::Sender`.
    let popup = state.active_popup.take();
    let PopupType::PluginInputDialog {
        title,
        input,
        cursor_idx,
        obscure,
        reply_tx,
    } = popup
    .ok_or(())?
    else {
        // Not the right variant — restore and return.
        state.active_popup = Some(PopupType::Info(String::new()));
        return Err(());
    };
    let mut new_input = input;
    let mut new_cursor = cursor_idx;
    match key.code {
        KeyCode::Char(c) => {
            new_input.push(c);
            new_cursor = new_input.chars().count();
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Backspace => {
            new_input.pop();
            new_cursor = new_input.chars().count();
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Left => {
            new_cursor = new_cursor.saturating_sub(1);
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Right => {
            let max = new_input.chars().count();
            new_cursor = (new_cursor + 1).min(max);
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Home => {
            new_cursor = 0;
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::End => {
            new_cursor = new_input.chars().count();
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Enter => {
            if let Some(tx) = reply_tx {
                let _ = tx.send(crate::plugin::manager::InputDialogResult {
                    value: new_input,
                    event: 1, // submitted
                });
            }
            state.active_popup = None;
            Ok(None)
        }
        KeyCode::Esc => {
            if let Some(tx) = reply_tx {
                let _ = tx.send(crate::plugin::manager::InputDialogResult {
                    value: new_input,
                    event: 2, // cancelled
                });
            }
            state.active_popup = None;
            Ok(None)
        }
        _ => {
            // Unhandled key: restore the popup unchanged.
            state.active_popup = Some(PopupType::PluginInputDialog {
                title,
                input: new_input,
                cursor_idx: new_cursor,
                obscure,
                reply_tx,
            });
            Err(())
        }
    }
}

fn handle_confirm(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.take();
    let PopupType::PluginConfirmDialog {
        title,
        msg,
        cursor_idx,
        reply_tx,
    } = popup
    .ok_or(())?
    else {
        state.active_popup = Some(PopupType::Info(String::new()));
        return Err(());
    };
    match key.code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            let new_idx = if cursor_idx == 0 { 1 } else { 0 };
            state.active_popup = Some(PopupType::PluginConfirmDialog {
                title,
                msg,
                cursor_idx: new_idx,
                reply_tx,
            });
            Ok(None)
        }
        KeyCode::Enter => {
            let confirmed = cursor_idx == 0;
            if let Some(tx) = reply_tx {
                let _ = tx.send(confirmed);
            }
            state.active_popup = None;
            Ok(None)
        }
        KeyCode::Esc => {
            if let Some(tx) = reply_tx {
                let _ = tx.send(false);
            }
            state.active_popup = None;
            Ok(None)
        }
        _ => {
            // Unhandled key: restore the popup unchanged.
            state.active_popup = Some(PopupType::PluginConfirmDialog {
                title,
                msg,
                cursor_idx,
                reply_tx,
            });
            Err(())
        }
    }
}

fn handle_which(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.take();
    let PopupType::PluginWhichPrompt {
        candidates,
        silent,
        reply_tx,
    } = popup
    .ok_or(())?
    else {
        state.active_popup = Some(PopupType::Info(String::new()));
        return Err(());
    };
    let key_str = key_event_to_string(key);
    if !key_str.is_empty() {
        for (idx, cand) in candidates.iter().enumerate() {
            if cand.on.iter().any(|k| k == &key_str) {
                if let Some(tx) = reply_tx {
                    // The Lua API returns 1-based indices, so add 1 here.
                    let _ = tx.send(Some(idx + 1));
                }
                state.active_popup = None;
                return Ok(None);
            }
        }
    }
    if matches!(key.code, KeyCode::Esc) {
        if let Some(tx) = reply_tx {
            let _ = tx.send(None);
        }
        state.active_popup = None;
        return Ok(None);
    }
    // Unhandled key (no match, not Esc): restore the popup.
    state.active_popup = Some(PopupType::PluginWhichPrompt {
        candidates,
        silent,
        reply_tx,
    });
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::PopupType;
    use crate::plugin::manager::{InputDialogResult, WhichCandidate};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn fresh_state() -> AppState {
        AppState::new(std::path::PathBuf::from("/"), std::path::PathBuf::from("/"))
    }

    #[test]
    fn test_input_dialog_typing_appends() {
        let mut state = fresh_state();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginInputDialog {
            title: "T".into(),
            input: "ab".into(),
            cursor_idx: 2,
            obscure: false,
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        match state.active_popup {
            Some(PopupType::PluginInputDialog { input, cursor_idx, .. }) => {
                assert_eq!(input, "abc");
                assert_eq!(cursor_idx, 3);
            }
            _ => panic!("input popup not preserved"),
        }
    }

    #[test]
    fn test_input_dialog_enter_sends_submitted() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginInputDialog {
            title: "T".into(),
            input: "hi".into(),
            cursor_idx: 2,
            obscure: false,
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        // Popup must be cleared.
        assert!(state.active_popup.is_none());
        // The receiver must see a Submitted result.
        let result = rx.try_recv().expect("message sent");
        assert_eq!(result.value, "hi");
        assert_eq!(result.event, 1);
    }

    #[test]
    fn test_input_dialog_esc_sends_cancelled() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginInputDialog {
            title: "T".into(),
            input: "x".into(),
            cursor_idx: 1,
            obscure: true,
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        assert!(state.active_popup.is_none());
        let result = rx.try_recv().expect("message sent");
        assert_eq!(result.event, 2);
    }

    #[test]
    fn test_confirm_dialog_enter_sends_true_when_yes() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginConfirmDialog {
            title: "T".into(),
            msg: "?".into(),
            cursor_idx: 0, // Yes
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        assert!(state.active_popup.is_none());
        assert_eq!(rx.try_recv().unwrap(), true);
    }

    #[test]
    fn test_confirm_dialog_enter_sends_false_when_no() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginConfirmDialog {
            title: "T".into(),
            msg: "?".into(),
            cursor_idx: 1, // No
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        assert_eq!(rx.try_recv().unwrap(), false);
    }

    #[test]
    fn test_confirm_dialog_toggle_with_arrow() {
        let mut state = fresh_state();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginConfirmDialog {
            title: "T".into(),
            msg: "?".into(),
            cursor_idx: 0,
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        match state.active_popup {
            Some(PopupType::PluginConfirmDialog { cursor_idx, .. }) => {
                assert_eq!(cursor_idx, 1);
            }
            _ => panic!("confirm popup not preserved"),
        }
    }

    #[test]
    fn test_which_prompt_matches_candidate_key() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginWhichPrompt {
            candidates: vec![
                WhichCandidate {
                    on: vec!["a".into()],
                    desc: Some("Apple".into()),
                },
                WhichCandidate {
                    on: vec!["b".into()],
                    desc: Some("Banana".into()),
                },
            ],
            silent: false,
            reply_tx: Some(tx),
        });
        // Simulate the user pressing 'b' — the resolver should
        // canonicalise this to "b" and the handler should match the
        // second candidate (1-based index = 2).
        let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        assert!(state.active_popup.is_none());
        assert_eq!(rx.try_recv().unwrap(), Some(2));
    }

    #[test]
    fn test_which_prompt_esc_sends_none() {
        let mut state = fresh_state();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginWhichPrompt {
            candidates: vec![WhichCandidate {
                on: vec!["a".into()],
                desc: None,
            }],
            silent: false,
            reply_tx: Some(tx),
        });
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let _ = handle(&mut state, key);
        assert!(state.active_popup.is_none());
        assert_eq!(rx.try_recv().unwrap(), None);
    }

    #[test]
    fn test_unrelated_key_restores_popup() {
        let mut state = fresh_state();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        state.active_popup = Some(PopupType::PluginInputDialog {
            title: "T".into(),
            input: "x".into(),
            cursor_idx: 1,
            obscure: false,
            reply_tx: Some(tx),
        });
        // F5 is not a handled key — the popup must be preserved.
        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
        let result = handle(&mut state, key);
        assert!(result.is_err());
        assert!(state.active_popup.is_some());
    }
}

