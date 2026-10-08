//! Keys of the multi-rename dialog: form focus / editing through
//! [`FormLayout`](crate::app::form::FormLayout); PgUp / PgDn scroll the
//! preview.

use crate::app::actions::fs_ops::multi_rename;
use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::popup::MultiRenameState as Dialog;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Preview rows moved by PgUp / PgDn.
const PREVIEW_PAGE: usize = 10;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::MultiRename(dialog)) = state.dialogs.top_mut() else {
        return Err(());
    };
    if dialog.running {
        return Ok(None);
    }
    if scroll_preview(dialog, key.code) {
        return Ok(None);
    }
    if dialog.focus == Dialog::ROW_CASE && matches!(key.code, KeyCode::Left | KeyCode::Right) {
        dialog.case = dialog.case.cycle(key.code == KeyCode::Right);
        dialog.refresh_preview();
        return Ok(None);
    }
    let mut focus = dialog.focus;
    let outcome = Dialog::FORM.handle(&mut focus, dialog.focused_field_mut(), &key);
    dialog.focus = focus;
    match outcome {
        FormKey::Activate(Dialog::BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => multi_rename::start(state),
        FormKey::Toggle(row) => {
            toggle(dialog, row);
            dialog.refresh_preview();
        }
        FormKey::Handled => dialog.refresh_preview(),
        FormKey::Other => {}
    }
    Ok(None)
}

/// Space on an option row.
fn toggle(dialog: &mut Dialog, row: usize) {
    match row {
        Dialog::ROW_REGEX => dialog.regex = !dialog.regex,
        Dialog::ROW_IGNORE_CASE => dialog.ignore_case = !dialog.ignore_case,
        Dialog::ROW_CASE => dialog.case = dialog.case.cycle(true),
        _ => {}
    }
}

/// PgUp / PgDn scroll the preview table (arrows move the focus between the
/// fields).
fn scroll_preview(dialog: &mut Dialog, code: KeyCode) -> bool {
    if !matches!(code, KeyCode::PageUp | KeyCode::PageDown) {
        return false;
    }
    let keys = ScrollKeys {
        vim: false,
        home_end: false,
        page: PREVIEW_PAGE,
    };
    let last = dialog.sources.len().saturating_sub(1);
    scroll_key(keys, code, &mut dialog.scroll, last)
}

#[cfg(test)]
mod tests;
