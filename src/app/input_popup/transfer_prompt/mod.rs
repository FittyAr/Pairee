//! Keys of the shared Copy / Move dialog ([`PopupType::TransferPrompt`]).
//! The operation-specific bits (labels, job kind) come from
//! [`crate::app::state::popup::TransferPromptOp`].

mod submit;

use crate::app::context::AppContext;
use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::state::{AppState, CopyMovePromptState as Prompt, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.top_mut() else {
        return Err(());
    };
    let row = prompt.cursor_idx;
    let on_input = row == Prompt::ROW_INPUT;
    match key.code {
        KeyCode::Up | KeyCode::BackTab => prompt.cursor_idx = wrap_prev(row, Prompt::ROW_COUNT),
        KeyCode::Down | KeyCode::Tab => prompt.cursor_idx = wrap_next(row, Prompt::ROW_COUNT),
        KeyCode::Right if on_input && prompt.input.cursor_at_end() => {
            if let Some(suggestion) =
                crate::fs::transfer::history::suggest_destination(prompt.input.text())
            {
                prompt.input.set_text(suggestion);
            }
        }
        KeyCode::Left | KeyCode::Right if Prompt::is_button(row) => {
            prompt.cursor_idx = cycle_button(row, key.code == KeyCode::Left);
        }
        KeyCode::Char(' ') if !on_input => prompt.toggle(row),
        _ if on_input && prompt.input.handle_key(&key).consumed() => {}
        // Editing keys outside the input row are swallowed.
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {}
        KeyCode::Enter => match row {
            Prompt::BUTTON_CANCEL => state.dialogs.clear(),
            Prompt::BUTTON_TREE => submit::open_tree_view(state),
            Prompt::BUTTON_FILTER => submit::open_filter_prompt(state),
            _ => submit::submit(state, context),
        },
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::F(10) => submit::open_tree_view(state),
        _ => return Err(()),
    }
    Ok(None)
}

/// Left / Right across the button bar, wrapping at both ends.
fn cycle_button(row: usize, left: bool) -> usize {
    let first = Prompt::BUTTON_SUBMIT;
    let count = Prompt::BUTTON_CANCEL - first + 1;
    let step = if left { wrap_prev } else { wrap_next };
    first + step(row - first, count)
}

#[cfg(test)]
mod tests;
