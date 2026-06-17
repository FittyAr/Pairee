use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};

pub fn handle(
    state: &mut AppState,
    _context: &mut AppContext,
) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let default_name = targets
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());
        let dest_dir = state.get_passive_panel().current_path.clone();
        state.active_popup = Some(PopupType::CompressPrompt {
            input: default_name,
            targets,
            dest_dir,
        });
    }
    true
}
