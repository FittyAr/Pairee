use crate::app::actions::fs_ops::rename as rename_action;
use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::state::popup::forms::RENAME_FORM;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

const BUTTON_CANCEL: usize = 2;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::RenamePrompt {
        input, cursor_idx, ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let field = (*cursor_idx == 0).then_some(input);
    match RENAME_FORM.handle(cursor_idx, field, &key) {
        FormKey::Activate(BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => {
            if let Some(PopupType::RenamePrompt {
                input,
                original,
                src_path,
                parent_dir,
                ..
            }) = state.dialogs.pop()
            {
                rename_action::commit(
                    state,
                    context,
                    input.into_text(),
                    original,
                    src_path,
                    parent_dir,
                );
            }
        }
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}
