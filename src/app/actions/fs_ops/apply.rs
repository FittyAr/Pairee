use crate::app::state::AppState;
use crate::app::state::popup::ApplyCommandPrompt;

pub fn handle(state: &mut AppState) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        state.dialogs.replace(ApplyCommandPrompt {
            input: Default::default(),
            targets,
        });
    }
    true
}
