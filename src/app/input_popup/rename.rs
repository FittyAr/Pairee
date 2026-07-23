use crate::app::actions::fs_ops::rename as rename_action;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const MAX_CURSOR_IDX: usize = 2; // 0 = input, 1 = OK, 2 = Cancel

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::RenamePrompt {
        input,
        original,
        src_path,
        parent_dir,
        cursor_idx,
    }) = state.active_popup.clone()
    else {
        return Err(());
    };

    let mut new_input = input.clone();
    let mut new_idx = cursor_idx;

    let update = |s: &mut AppState, i: String, idx: usize| {
        s.active_popup = Some(PopupType::RenamePrompt {
            input: i,
            original: original.clone(),
            src_path: src_path.clone(),
            parent_dir: parent_dir.clone(),
            cursor_idx: idx,
        });
    };

    match key.code {
        KeyCode::Esc => {
            state.active_popup = None;
            Ok(None)
        }
        KeyCode::Up | KeyCode::BackTab => {
            new_idx = if new_idx > 0 {
                new_idx - 1
            } else {
                MAX_CURSOR_IDX
            };
            update(state, new_input, new_idx);
            Ok(None)
        }
        KeyCode::Down | KeyCode::Tab => {
            new_idx = if new_idx < MAX_CURSOR_IDX {
                new_idx + 1
            } else {
                0
            };
            update(state, new_input, new_idx);
            Ok(None)
        }
        KeyCode::Left | KeyCode::Right => {
            if new_idx >= 1 {
                new_idx = if new_idx == 1 { 2 } else { 1 };
                update(state, new_input, new_idx);
            }
            Ok(None)
        }
        KeyCode::Backspace => {
            if new_idx == 0 {
                new_input.pop();
                update(state, new_input, new_idx);
            }
            Ok(None)
        }
        KeyCode::Char(c) => {
            if new_idx == 0 {
                new_input.push(c);
                update(state, new_input, new_idx);
            }
            Ok(None)
        }
        KeyCode::Enter => {
            if new_idx == 2 {
                state.active_popup = None;
                return Ok(None);
            }
            rename_action::commit(state, context, new_input, original, src_path, parent_dir);
            Ok(None)
        }
        _ => Ok(None),
    }
}
