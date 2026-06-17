use crate::app::state::{AppState, PopupType};

pub fn handle(state: &mut AppState) -> bool {
    state.active_popup = Some(PopupType::MkDirPrompt {
        input: String::new(),
        cursor_idx: 0,
        process_multiple: false,
    });
    true
}
