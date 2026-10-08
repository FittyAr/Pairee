//! History lists (commands, file views, and visited folders): one handler,
//! the list kind ([`HistoryKind`]) selects what Enter does.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::history::HistoryKind;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some((kind, cursor_idx, len)) = state
        .dialogs
        .top_mut()
        .and_then(PopupType::history_list_mut)
    else {
        return Err(());
    };
    match list_key(ListKeys::FULL, key.code, cursor_idx, len) {
        ListKey::Moved | ListKey::Other if key.code != KeyCode::Delete => {}
        ListKey::Close => state.dialogs.clear(),
        ListKey::Activate(idx) => {
            let popup = state.dialogs.pop();
            state.dialogs.clear();
            if let Some(popup) = popup {
                activate(state, context, popup, idx);
            }
        }
        _ if key.modifiers.contains(KeyModifiers::ALT) => clear_all(state, context, kind),
        _ => {
            let idx = *cursor_idx;
            if idx < len {
                remove_entry(state, kind, idx);
            }
        }
    }
    Ok(None)
}

/// Enter: run the command line, view the file or open the folder.
fn activate(state: &mut AppState, context: &AppContext, popup: PopupType, idx: usize) {
    match popup {
        PopupType::CommandHistoryList { mut entries, .. } if idx < entries.len() => {
            state.cli_input = entries.swap_remove(idx);
        }
        PopupType::FileViewHistoryList { mut entries, .. } if idx < entries.len() => {
            let path = entries.swap_remove(idx);
            state.open_viewer(path, context.config.settings.image_preview_enabled, false);
        }
        PopupType::FoldersHistoryList { mut entries, .. } if idx < entries.len() => {
            state
                .get_active_panel_mut()
                .open_path(entries.swap_remove(idx));
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        _ => {}
    }
}

/// Alt+Del: clear the whole list (asking first when configured to).
fn clear_all(state: &mut AppState, context: &AppContext, kind: HistoryKind) {
    if context
        .config
        .settings
        .confirmations
        .confirm_clear_history_list
    {
        state.dialogs.replace(PopupType::ConfirmClearHistory {
            history_type: kind.key().to_string(),
        });
    } else {
        state.history.clear(kind);
        state.history.save();
        state.dialogs.clear();
    }
}

/// Del: remove the entry under the cursor.
fn remove_entry(state: &mut AppState, kind: HistoryKind, idx: usize) {
    let remaining = state.history.remove(kind, idx);
    state.history.save();
    if remaining == 0 {
        state.dialogs.clear();
    } else {
        let list = kind.list_popup(&state.history, idx.min(remaining - 1));
        state.dialogs.replace(list);
    }
}
