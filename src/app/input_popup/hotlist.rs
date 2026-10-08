use crate::app::context::AppContext;
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
        mut entries,
        cursor_idx,
    }) = state.dialogs.top().cloned()
    else {
        return Err(());
    };

    match key.code {
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Up | KeyCode::Down => {
            let new_idx = step_cursor(cursor_idx, entries.len(), key.code == KeyCode::Up);
            state.dialogs.replace(PopupType::Hotlist {
                entries,
                cursor_idx: new_idx,
            });
        }
        KeyCode::Enter => {
            if let Some(entry) = entries.get(cursor_idx) {
                let target = entry.path.clone();
                state.dialogs.clear();
                state.jump_active_panel_to(target, context.config.settings.show_hidden);
            }
        }
        KeyCode::Insert | KeyCode::Char('+') => {
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
        KeyCode::Delete | KeyCode::Char('-') => {
            if cursor_idx < entries.len() {
                entries.remove(cursor_idx);
                let idx = cursor_idx.min(entries.len().saturating_sub(1));
                persist(state, entries, idx);
            }
        }
        _ => return Err(()),
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

/// Wrapping cursor movement shared by the hotlist and folder shortcut dialogs.
pub fn step_cursor(idx: usize, len: usize, up: bool) -> usize {
    if len == 0 {
        0
    } else if up {
        if idx == 0 { len - 1 } else { idx - 1 }
    } else if idx + 1 >= len {
        0
    } else {
        idx + 1
    }
}

#[cfg(test)]
mod tests {
    use super::step_cursor;

    #[test]
    fn cursor_wraps_both_ways() {
        assert_eq!(step_cursor(0, 3, true), 2);
        assert_eq!(step_cursor(2, 3, false), 0);
        assert_eq!(step_cursor(1, 3, false), 2);
        assert_eq!(step_cursor(0, 0, true), 0);
    }
}
