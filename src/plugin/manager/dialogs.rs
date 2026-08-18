//! Opens real TUI dialogs for plugin confirm / input / which requests.
//!
//! Oneshot replies live in [`PluginHostState::pending_dialog`] because
//! [`PopupType`] is `Clone` and senders are not.

use super::request::{DialogPosition, WhichCandidate};
use crate::app::state::{AppState, PendingPluginReply, PopupType};
use tokio::sync::oneshot;

pub fn is_plugin_dialog(popup: &PopupType) -> bool {
    matches!(
        popup,
        PopupType::PluginConfirm { .. }
            | PopupType::PluginInput { .. }
            | PopupType::PluginWhich { .. }
    )
}

/// If a reply is pending but the plugin dialog is gone, cancel the waiter.
pub fn settle_orphaned_plugin_dialogs(state: &mut AppState) {
    let orphaned = state.plugins.pending_dialog.is_some()
        && !state.dialogs.top().is_some_and(is_plugin_dialog);
    if orphaned {
        state.plugins.cancel_pending();
    }
}

pub fn open_confirm(
    state: &mut AppState,
    title: String,
    msg: String,
    position: Option<DialogPosition>,
    reply_tx: oneshot::Sender<bool>,
) {
    state.plugins.cancel_pending();
    state.plugins.pending_dialog = Some(PendingPluginReply::Confirm(reply_tx));
    state.dialogs.replace(PopupType::PluginConfirm {
        title,
        msg,
        cursor_idx: 0,
        position,
    });
    state.mark_ui_dirty();
}

pub fn open_input(
    state: &mut AppState,
    title: String,
    default: String,
    obscure: bool,
    position: Option<DialogPosition>,
    reply: PendingPluginReply,
) {
    state.plugins.cancel_pending();
    state.plugins.pending_dialog = Some(reply);
    state.dialogs.replace(PopupType::PluginInput {
        title,
        input: default,
        obscure,
        position,
    });
    state.mark_ui_dirty();
}

pub fn open_which(
    state: &mut AppState,
    candidates: Vec<WhichCandidate>,
    silent: bool,
    reply_tx: oneshot::Sender<Option<usize>>,
) {
    state.plugins.cancel_pending();
    state.plugins.pending_dialog = Some(PendingPluginReply::Which(reply_tx));
    state.dialogs.replace(PopupType::PluginWhich {
        candidates,
        silent,
        position: None,
    });
    state.mark_ui_dirty();
}

/// Compare a pressed key (keybinds display form) to a Lua `on` spec.
///
/// Accepts both `Ctrl+c` / `Down` (resolver) and `<C-c>` / `<Down>` (Lua).
pub fn key_matches_spec(pressed: &str, spec: &str) -> bool {
    normalize_key_spec(pressed) == normalize_key_spec(spec)
}

pub fn normalize_key_spec(raw: &str) -> String {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(trimmed)
        .trim();

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut rest = inner.to_string();

    loop {
        let lower = rest.to_ascii_lowercase();
        if let Some(next) = lower
            .strip_prefix("ctrl+")
            .or_else(|| lower.strip_prefix("ctrl-"))
            .or_else(|| lower.strip_prefix("control+"))
            .or_else(|| lower.strip_prefix("control-"))
            .or_else(|| lower.strip_prefix("c-"))
        {
            ctrl = true;
            rest = next.to_string();
            continue;
        }
        if let Some(next) = lower
            .strip_prefix("alt+")
            .or_else(|| lower.strip_prefix("alt-"))
            .or_else(|| lower.strip_prefix("a-"))
        {
            alt = true;
            rest = next.to_string();
            continue;
        }
        if let Some(next) = lower
            .strip_prefix("shift+")
            .or_else(|| lower.strip_prefix("shift-"))
            .or_else(|| lower.strip_prefix("s-"))
        {
            shift = true;
            rest = next.to_string();
            continue;
        }
        break;
    }

    let key = rest.to_ascii_lowercase();
    let mut out = String::new();
    if ctrl {
        out.push_str("ctrl+");
    }
    if alt {
        out.push_str("alt+");
    }
    if shift {
        out.push_str("shift+");
    }
    out.push_str(&key);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manager::InputDialogResult;
    use std::path::PathBuf;

    fn test_state() -> AppState {
        AppState::new(PathBuf::from("."), PathBuf::from("."))
    }

    #[test]
    fn normalize_lua_and_resolver_ctrl_c() {
        assert_eq!(normalize_key_spec("<C-c>"), "ctrl+c");
        assert_eq!(normalize_key_spec("Ctrl+c"), "ctrl+c");
        assert_eq!(normalize_key_spec("Ctrl+C"), "ctrl+c");
        assert!(key_matches_spec("Ctrl+c", "<C-c>"));
    }

    #[test]
    fn normalize_arrow_and_plain_char() {
        assert_eq!(normalize_key_spec("<Down>"), "down");
        assert_eq!(normalize_key_spec("Down"), "down");
        assert!(key_matches_spec("Down", "<Down>"));
        assert!(key_matches_spec("a", "a"));
        assert!(key_matches_spec("A", "a"));
        assert!(!key_matches_spec("a", "b"));
    }

    #[test]
    fn open_confirm_stores_pending_and_popup() {
        let mut state = test_state();
        let (tx, mut rx) = oneshot::channel();
        open_confirm(
            &mut state,
            "Overwrite?".into(),
            "file exists".into(),
            None,
            tx,
        );
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::PluginConfirm { title, .. }) if title == "Overwrite?"
        ));
        assert!(matches!(
            state.plugins.pending_dialog,
            Some(PendingPluginReply::Confirm(_))
        ));
        // Reply is not sent until the user answers.
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn opening_second_dialog_cancels_the_first() {
        let mut state = test_state();
        let (tx1, rx1) = oneshot::channel();
        open_confirm(&mut state, "one".into(), "a".into(), None, tx1);
        let (tx2, _rx2) = oneshot::channel();
        open_confirm(&mut state, "two".into(), "b".into(), None, tx2);
        assert!(!rx1.blocking_recv().unwrap());
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::PluginConfirm { title, .. }) if title == "two"
        ));
    }

    #[test]
    fn settle_cancels_when_popup_was_replaced() {
        let mut state = test_state();
        let (tx, rx) = oneshot::channel::<InputDialogResult>();
        open_input(
            &mut state,
            "Name".into(),
            "x".into(),
            false,
            None,
            PendingPluginReply::Input(tx),
        );
        state.dialogs.replace(PopupType::Info("gone".into()));
        settle_orphaned_plugin_dialogs(&mut state);
        let result = rx.blocking_recv().unwrap();
        assert_eq!(result.event, 2);
        assert!(state.plugins.pending_dialog.is_none());
    }

    #[test]
    fn settle_keeps_pending_while_dialog_is_open() {
        let mut state = test_state();
        let (tx, mut rx) = oneshot::channel();
        open_confirm(&mut state, "t".into(), "m".into(), None, tx);
        settle_orphaned_plugin_dialogs(&mut state);
        assert!(state.plugins.pending_dialog.is_some());
        assert!(rx.try_recv().is_err());
    }
}
