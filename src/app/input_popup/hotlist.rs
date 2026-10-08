use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::config::bookmarks::{self, HotlistEntry};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Directory hotlist: Enter jumps, Ins adds the active folder, Del removes the entry.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::Hotlist {
        entries,
        cursor_idx,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match list_key(ListKeys::ARROWS, key.code, cursor_idx, entries.len()) {
        ListKey::Moved => {}
        ListKey::Close => state.dialogs.clear(),
        ListKey::Activate(idx) => {
            if let Some(target) = entries.get(idx).map(|e| e.path.clone()) {
                state.dialogs.clear();
                state.jump_active_panel_to(target, context.config.settings.show_hidden);
            }
        }
        ListKey::Other => match key.code {
            KeyCode::Insert | KeyCode::Char('+') => {
                let mut entries = std::mem::take(entries);
                let path = state.get_active_panel().current_path.clone();
                let idx = match entries.iter().position(|e| e.path == path) {
                    Some(existing) => existing,
                    None => {
                        entries.push(HotlistEntry {
                            name: bookmarks::entry_name_for(&path),
                            path,
                        });
                        entries.len() - 1
                    }
                };
                persist(state, entries, idx);
            }
            KeyCode::Delete | KeyCode::Char('-') if *cursor_idx < entries.len() => {
                let mut entries = std::mem::take(entries);
                entries.remove(*cursor_idx);
                let idx = (*cursor_idx).min(entries.len().saturating_sub(1));
                persist(state, entries, idx);
            }
            KeyCode::Delete | KeyCode::Char('-') => {}
            _ => return Err(()),
        },
    }
    Ok(None)
}

fn persist(state: &mut AppState, entries: Vec<HotlistEntry>, cursor_idx: usize) {
    let result = bookmarks::save_hotlist(&entries);
    state.dialogs.replace(PopupType::Hotlist {
        entries,
        cursor_idx,
    });
    if let Err(e) = result {
        state.dialogs.push(PopupType::Error(
            t("error_save_bookmarks").replace("{}", &e.to_string()),
        ));
    }
}
