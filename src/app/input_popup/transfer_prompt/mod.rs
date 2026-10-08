//! Keys of the shared Copy / Move dialog ([`PopupType::TransferPrompt`]).
//! The operation-specific bits (labels, job kind) come from
//! [`crate::app::state::popup::TransferPromptOp`].

mod submit;

use crate::app::context::AppContext;
use crate::app::form::FormKey;
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
    // Right at the end of the destination accepts the history suggestion.
    if key.code == KeyCode::Right
        && prompt.cursor_idx == Prompt::ROW_INPUT
        && prompt.input.cursor_at_end()
    {
        if let Some(suggestion) =
            crate::fs::transfer::history::suggest_destination(prompt.input.text())
        {
            prompt.input.set_text(suggestion);
        }
        return Ok(None);
    }
    let field = (prompt.cursor_idx == Prompt::ROW_INPUT).then_some(&mut prompt.input);
    match Prompt::FORM.handle(&mut prompt.cursor_idx, field, &key) {
        FormKey::Toggle(row) => prompt.toggle(row),
        FormKey::Activate(Prompt::BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(Prompt::BUTTON_TREE) => submit::open_tree_view(state),
        FormKey::Activate(Prompt::BUTTON_FILTER) => submit::open_filter_prompt(state),
        FormKey::Activate(_) => submit::submit(state, context),
        FormKey::Other if key.code == KeyCode::F(10) => submit::open_tree_view(state),
        FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
