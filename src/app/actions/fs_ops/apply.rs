use crate::app::state::{AppState, PopupType};

pub fn handle(
    state: &mut AppState,
) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        state.active_popup = Some(PopupType::ApplyCommandPrompt {
            input: String::new(),
            targets,
        });
    }
    true
}
