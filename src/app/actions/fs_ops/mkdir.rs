use crate::app::state::{AppState, PopupType};

pub fn handle(state: &mut AppState) -> bool {
    state.dialogs.replace(PopupType::MkDirPrompt {
        input: Default::default(),
        cursor_idx: 0,
        process_multiple: false,
    });
    true
}
