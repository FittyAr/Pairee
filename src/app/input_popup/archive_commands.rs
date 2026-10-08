use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::ArchiveCommandsMenu {
        archive_path,
        items,
        cursor_idx,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let chosen = match list_key(ListKeys::ARROWS_VIM, key.code, cursor_idx, items.len()) {
        ListKey::Moved => return Ok(None),
        ListKey::Close => {
            state.dialogs.clear();
            return Ok(None);
        }
        ListKey::Activate(idx) => idx,
        // 1-4 pick an entry directly.
        ListKey::Other => match key.code {
            KeyCode::Char(c @ '1'..='4') if (c as usize - '1' as usize) < items.len() => {
                c as usize - '1' as usize
            }
            KeyCode::Char('1'..='4') => return Ok(None),
            _ => return Err(()),
        },
    };
    let archive_path = archive_path.clone();
    execute_option(state, &archive_path, chosen);
    Ok(None)
}

fn execute_option(state: &mut AppState, archive_path: &Path, cursor_idx: usize) {
    state.dialogs.clear();
    match cursor_idx {
        0 => {
            // List contents
            match crate::fs::archive::list_archive_files(archive_path) {
                Ok(list) => {
                    let viewer =
                        crate::ui::viewer::ViewerState::from_text(archive_path.to_path_buf(), list);
                    state.push_screen(Screen::Viewer(viewer));
                }
                Err(e) => {
                    state
                        .dialogs
                        .replace(PopupType::Error(format!("Failed to list archive: {}", e)));
                }
            }
        }
        1 => {
            // Test integrity
            match crate::fs::archive::list_archive_files(archive_path) {
                Ok(_) => {
                    state
                        .dialogs
                        .replace(PopupType::Info(crate::config::localization::t(
                            "archive_test_ok",
                        )));
                }
                Err(e) => {
                    state.dialogs.replace(PopupType::Error(format!(
                        "Archive integrity check failed: {}",
                        e
                    )));
                }
            }
        }
        2 | 3 => {
            // Extract
            let dest = if cursor_idx == 2 {
                state.get_active_panel().current_path.clone()
            } else {
                state.get_passive_panel().current_path.clone()
            };
            crate::fs::transfer::submit_simple(
                state,
                crate::fs::transfer::job::TransferOperation::Extract,
                vec![archive_path.to_path_buf()],
                dest,
                crate::fs::transfer::options::TransferOptions::default(),
                None,
                None,
            );
        }
        _ => {}
    }
}
