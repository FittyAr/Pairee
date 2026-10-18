use crate::app::actions::fs_ops::extract;
use crate::app::context::AppContext;
use crate::app::dialog_loads::{ArchiveRead, read_archive};
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::AppState;
use crate::app::state::popup::ArchiveCommandsMenu;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(ArchiveCommandsMenu {
        archive_path,
        items,
        cursor_idx,
    }) = state.dialogs.top_mut_as::<ArchiveCommandsMenu>()
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
    execute_option(state, context, &archive_path, chosen);
    Ok(None)
}

fn execute_option(
    state: &mut AppState,
    context: &AppContext,
    archive_path: &Path,
    cursor_idx: usize,
) {
    state.dialogs.clear();
    match cursor_idx {
        0 => read_archive(state, archive_path.to_path_buf(), ArchiveRead::List),
        1 => read_archive(state, archive_path.to_path_buf(), ArchiveRead::Test),
        2 => {
            let dest = state.get_active_panel().current_path.clone();
            extract::extract_to(state, context, archive_path, dest);
        }
        3 => extract::extract_to_other_panel(state, context, archive_path),
        _ => {}
    }
}

impl Dialog for ArchiveCommandsMenu {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
}
