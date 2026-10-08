//! SSH Connection popup input handler.

pub mod actions;

use crate::app::context::AppContext;
use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::state::popup::SshField;
use crate::app::state::{AppState, PopupType, SshConnectPromptState as Prompt};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const LAST_FIELD: usize = 6;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SshConnectPrompt(prompt)) = state.dialogs.top_mut() else {
        return Err(());
    };
    let presets = &context.config.settings.ssh_presets;
    let row = prompt.cursor_idx;
    let is_button = row >= Prompt::BUTTON_CONNECT;
    match key.code {
        KeyCode::Up | KeyCode::Down if row == Prompt::ROW_PRESETS => {
            if !presets.is_empty() {
                let current = prompt.selected_preset_idx.unwrap_or(0);
                let next = if key.code == KeyCode::Up {
                    wrap_prev(current, presets.len())
                } else {
                    wrap_next(current, presets.len())
                };
                prompt.load_preset(next, &presets[next]);
            }
        }
        KeyCode::Up => prompt.cursor_idx = if is_button { LAST_FIELD } else { row - 1 },
        KeyCode::Down => {
            prompt.cursor_idx = match row {
                LAST_FIELD => Prompt::BUTTON_CONNECT,
                _ if is_button => Prompt::ROW_PRESETS,
                _ => row + 1,
            }
        }
        KeyCode::Left | KeyCode::Right if is_button => {
            let first = Prompt::BUTTON_CONNECT;
            let count = Prompt::ROW_COUNT - first;
            let step = if key.code == KeyCode::Left {
                wrap_prev
            } else {
                wrap_next
            };
            prompt.cursor_idx = first + step(row - first, count);
        }
        KeyCode::Right if row == Prompt::ROW_PRESETS => prompt.cursor_idx = SshField::Name.row(),
        // Left at the start of a field goes back to the preset list.
        KeyCode::Left
            if prompt
                .field_at(row)
                .is_some_and(|(_, field)| field.cursor() == 0) =>
        {
            prompt.cursor_idx = Prompt::ROW_PRESETS;
        }
        KeyCode::Tab => prompt.cursor_idx = wrap_next(row, Prompt::ROW_COUNT),
        KeyCode::BackTab => prompt.cursor_idx = wrap_prev(row, Prompt::ROW_COUNT),
        KeyCode::Enter => actions::handle_enter(state, context),
        KeyCode::Esc => state.dialogs.clear(),
        // Only digits go into the port.
        KeyCode::Char(c) if row == SshField::Port.row() && !c.is_ascii_digit() => {}
        _ => match prompt.field_at(row) {
            Some((_, field)) => {
                if !field.handle_key(&key).consumed() {
                    return Err(());
                }
            }
            None if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace) => {}
            None => return Err(()),
        },
    }
    Ok(None)
}
