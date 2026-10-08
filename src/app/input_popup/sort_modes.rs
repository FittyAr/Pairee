use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType, SortField};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Sort fields in dialog order; the row after them toggles reverse order.
const FIELDS: [SortField; 5] = [
    SortField::Name,
    SortField::Extension,
    SortField::Size,
    SortField::Date,
    SortField::Unsorted,
];

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SortModesDialog {
        current,
        reverse,
        cursor_idx,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let rows = FIELDS.len() + 1;
    let row = *cursor_idx;
    let field = FIELDS.get(row).copied();
    let close = match list_key(ListKeys::ARROWS, key.code, cursor_idx, rows) {
        ListKey::Moved => return Ok(None),
        ListKey::Close => {
            state.dialogs.clear();
            return Ok(None);
        }
        ListKey::Activate(_) => true,
        ListKey::Other if key.code == KeyCode::Char(' ') => false,
        ListKey::Other => return Err(()),
    };
    // Enter applies and closes; Space applies and keeps the dialog open.
    let reverse_now = *reverse;
    if !close {
        match field {
            Some(f) => *current = f,
            None => *reverse = !reverse_now,
        }
    }
    let panel = state.get_active_panel_mut();
    match field {
        Some(f) => {
            panel.sort_field = f;
            if close {
                panel.sort_reverse = reverse_now;
            }
        }
        None => panel.sort_reverse = !reverse_now,
    }
    if close {
        state.dialogs.clear();
    }
    state.refresh_both_panels(context.config.settings.show_hidden);
    Ok(None)
}
