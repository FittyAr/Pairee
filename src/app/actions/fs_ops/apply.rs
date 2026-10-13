use crate::app::state::popup::ApplyCommandPrompt;
use crate::app::state::{AppState, PopupType};

pub fn handle(state: &mut AppState) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        state
            .dialogs
            .replace(PopupType::ApplyCommandPrompt(ApplyCommandPrompt {
                input: Default::default(),
                targets,
            }));
    }
    true
}
