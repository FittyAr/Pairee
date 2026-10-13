//! Keys of the connection manager dialog.

pub mod actions;

use crate::app::context::AppContext;
use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::state::popup::{ConnectionField, Dialog, KeyHandler};
use crate::app::state::{AppState, ConnectionPromptState as Prompt, PopupType};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::ConnectionPrompt(prompt)) = state.dialogs.top_mut() else {
        return Err(());
    };
    let saved = &context.config.settings.connections;
    let row = prompt.cursor_idx;
    let is_button = row >= Prompt::BUTTON_CONNECT;
    let on_kind = row == ConnectionField::Kind.row();
    match key.code {
        KeyCode::Up | KeyCode::Down if row == Prompt::ROW_LIST => {
            if !saved.is_empty() {
                let current = prompt.selected_idx.unwrap_or(0);
                let next = if key.code == KeyCode::Up {
                    wrap_prev(current, saved.len())
                } else {
                    wrap_next(current, saved.len())
                };
                prompt.load(next, &saved[next]);
            }
        }
        KeyCode::Up => {
            prompt.cursor_idx = if is_button {
                Prompt::LAST_FIELD
            } else {
                row - 1
            }
        }
        KeyCode::Down => {
            prompt.cursor_idx = match row {
                Prompt::LAST_FIELD => Prompt::BUTTON_CONNECT,
                _ if is_button => Prompt::ROW_LIST,
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
        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if on_kind => {
            prompt.cycle_kind(key.code != KeyCode::Left);
        }
        KeyCode::Right if row == Prompt::ROW_LIST => {
            prompt.cursor_idx = ConnectionField::Name.row();
        }
        // Left at the start of a field goes back to the connection list.
        KeyCode::Left
            if prompt
                .text_field_at(row)
                .is_some_and(|field| field.cursor() == 0) =>
        {
            prompt.cursor_idx = Prompt::ROW_LIST;
        }
        KeyCode::Tab => prompt.cursor_idx = wrap_next(row, Prompt::ROW_COUNT),
        KeyCode::BackTab => prompt.cursor_idx = wrap_prev(row, Prompt::ROW_COUNT),
        KeyCode::Enter => actions::handle_enter(state, context),
        KeyCode::Esc => state.dialogs.clear(),
        // Only digits go into the port.
        KeyCode::Char(c) if row == ConnectionField::Port.row() && !c.is_ascii_digit() => {}
        _ => match prompt.text_field_at(row) {
            Some(field) => {
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

impl Dialog for Prompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        let row = self.cursor_idx;
        self.text_field_at(row)
    }
}
