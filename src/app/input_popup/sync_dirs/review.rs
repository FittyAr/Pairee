//! Review of the differences: move, change the action per item, apply.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::SyncDialog;
use crate::app::state::{AppState, PopupType};
use crate::fs::sync::SyncAction;
use crate::fs::sync::plan::{cycle_action, delete_action, set_action};
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle(state: &mut AppState, key: KeyEvent, context: &AppContext) {
    let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() else {
        return;
    };
    let Some(review) = dialog.review.as_mut() else {
        return;
    };
    if review.confirming {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y' | 'Y' | 's' | 'S') => apply(state, context),
            KeyCode::Esc | KeyCode::Char('n' | 'N') => review.confirming = false,
            _ => {}
        }
        return;
    }
    let len = review.visible().len();
    match list_key(ListKeys::FULL, key.code, &mut review.cursor, len) {
        ListKey::Moved => {}
        ListKey::Close => dialog.review = None,
        // Plans that delete anything need an explicit confirmation.
        ListKey::Activate(_) if review.summary().delete > 0 => review.confirming = true,
        ListKey::Activate(_) => apply(state, context),
        ListKey::Other => item_key(dialog, key.code),
    }
}

/// Per-item keys (and the review-wide toggles).
fn item_key(dialog: &mut SyncDialog, code: KeyCode) {
    if code == KeyCode::Tab {
        dialog.cycle_direction();
        return;
    }
    let Some(review) = dialog.review.as_mut() else {
        return;
    };
    if matches!(code, KeyCode::Char('e' | 'E')) {
        review.toggle_equal();
        return;
    }
    let Some(item) = review.current_mut() else {
        return;
    };
    match code {
        KeyCode::Right | KeyCode::Char('>') => {
            set_action(item, SyncAction::CopyToRight);
        }
        KeyCode::Left | KeyCode::Char('<') => {
            set_action(item, SyncAction::CopyToLeft);
        }
        KeyCode::Delete | KeyCode::Char('d' | 'D') => {
            if let Some(action) = delete_action(item) {
                set_action(item, action);
            }
        }
        KeyCode::Char('s' | 'S' | '=') => {
            set_action(item, SyncAction::Skip);
        }
        KeyCode::Char(' ') => cycle_action(item),
        _ => {}
    }
}

/// Closes the dialog and queues the plan on the Transfer Engine.
fn apply(state: &mut AppState, context: &AppContext) {
    if let Some(PopupType::SyncDirs(dialog)) = state.dialogs.pop()
        && let Some(review) = dialog.review
    {
        let mask = dialog.options.effective_mask();
        crate::app::sync::apply_review(state, context, &review.items, &mask);
    }
}
