//! Keys of the configuration dialog. The rows of every tab come from the
//! data tables in [`crate::app::config_rows`]; this module only moves the
//! focus, edits text rows and applies / cancels.

pub mod apply;

use crate::app::config_rows::{Activation, Row, RowCtx, TAB_KEYS, tab_rows};
use crate::app::context::AppContext;
use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::state::{AppState, ConfigurationDialogState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let custom_bindings = context.config.keybindings.custom_bindings.clone();
    let Some(PopupType::ConfigurationDialog(dlg)) = state.dialogs.top_mut() else {
        return Err(());
    };
    let rows = tab_rows(
        dlg.active_tab,
        &RowCtx {
            settings: &dlg.settings,
            custom_bindings: &custom_bindings,
        },
    );
    if dlg.edit.is_some() {
        edit_key(dlg, &rows, &key);
        return Ok(None);
    }
    // Rows, then the OK and Cancel buttons.
    let ok = rows.len();
    let cancel = ok + 1;
    if !is_selectable(&rows, dlg.cursor_idx) {
        dlg.cursor_idx = next_selectable(&rows, dlg.cursor_idx, wrap_next);
    }

    match key.code {
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Tab => dlg.focus_on_tabs = !dlg.focus_on_tabs,
        KeyCode::BackTab | KeyCode::Left => dlg.focus_on_tabs = true,
        KeyCode::Right => dlg.focus_on_tabs = false,
        KeyCode::Up | KeyCode::Down if dlg.focus_on_tabs => {
            let step = if key.code == KeyCode::Up {
                wrap_prev
            } else {
                wrap_next
            };
            let tab = step(dlg.active_tab, TAB_KEYS.len());
            open_tab(dlg, tab, &custom_bindings);
        }
        KeyCode::Up => dlg.cursor_idx = next_selectable(&rows, dlg.cursor_idx, wrap_prev),
        KeyCode::Down => dlg.cursor_idx = next_selectable(&rows, dlg.cursor_idx, wrap_next),
        KeyCode::Char(' ') | KeyCode::Enter if dlg.focus_on_tabs => dlg.focus_on_tabs = false,
        KeyCode::Char(' ') | KeyCode::Enter if dlg.cursor_idx == cancel => state.dialogs.clear(),
        KeyCode::Char(' ') | KeyCode::Enter if dlg.cursor_idx == ok => apply(state, context),
        KeyCode::Char(' ') | KeyCode::Enter => {
            let Some(setting) = rows.get(dlg.cursor_idx).and_then(Row::setting) else {
                return Ok(None);
            };
            match setting.activate(&mut dlg.settings, context) {
                Activation::Changed => {}
                Activation::StartEdit(field) => dlg.edit = Some(field),
                // Info panels open over the dialog; other dialogs replace it.
                Activation::Open(popup) => match *popup {
                    info @ (PopupType::Info(_) | PopupType::InfoPanel { .. }) => {
                        state.dialogs.push(info)
                    }
                    other => state.dialogs.replace(other),
                },
            }
        }
        KeyCode::F(9) => apply(state, context),
        KeyCode::Char(c) => {
            if let Some(tab) = tab_for_hotkey(c) {
                dlg.focus_on_tabs = false;
                open_tab(dlg, tab, &custom_bindings);
            }
        }
        _ => {}
    }
    Ok(None)
}

/// Keys while a text row is being edited.
fn edit_key(dlg: &mut ConfigurationDialogState, rows: &[Row], key: &KeyEvent) {
    let Some(field) = dlg.edit.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc => dlg.edit = None,
        KeyCode::Enter => {
            if let Some(setting) = rows.get(dlg.cursor_idx).and_then(Row::setting) {
                setting.commit_edit(&mut dlg.settings, field.text());
            }
            dlg.edit = None;
        }
        _ => {
            field.handle_key(key);
        }
    }
}

/// Applies the edited settings and closes the dialog.
fn apply(state: &mut AppState, context: &mut AppContext) {
    if let Some(PopupType::ConfigurationDialog(dlg)) = state.dialogs.pop() {
        apply::apply_settings(state, context, *dlg.settings);
    }
}

/// Switches to `tab`, focusing its first setting.
fn open_tab(
    dlg: &mut ConfigurationDialogState,
    tab: usize,
    custom_bindings: &std::collections::HashMap<String, String>,
) {
    dlg.active_tab = tab;
    let rows = tab_rows(
        tab,
        &RowCtx {
            settings: &dlg.settings,
            custom_bindings,
        },
    );
    dlg.cursor_idx = (0..rows.len())
        .find(|&i| rows[i].is_selectable())
        .unwrap_or(rows.len());
}

/// Setting rows and the two buttons after them are selectable.
fn is_selectable(rows: &[Row], idx: usize) -> bool {
    rows.get(idx).is_none_or(Row::is_selectable)
}

/// The next selectable index from `idx` in the direction of `step`.
fn next_selectable(rows: &[Row], idx: usize, step: fn(usize, usize) -> usize) -> usize {
    let count = rows.len() + 2;
    let mut i = step(idx, count);
    while !is_selectable(rows, i) {
        i = step(i, count);
    }
    i
}

/// Tab whose title hotkey (`&x` in the translation) is `c`.
fn tab_for_hotkey(c: char) -> Option<usize> {
    let c = c.to_ascii_lowercase();
    TAB_KEYS.iter().position(|key| {
        crate::ui::hotkey::parse_hotkey(&crate::config::localization::t(key)).hotkey == Some(c)
    })
}
