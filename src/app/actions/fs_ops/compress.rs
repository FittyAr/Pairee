use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::CompressPrompt;

pub fn handle(state: &mut AppState, _context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let default_name = targets
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());
        // The archive is written with local tools: into the other panel's
        // folder when it is on the local disk, else beside the files.
        let passive = state.get_passive_panel();
        let dest_dir = if passive.source.is_local() {
            passive.current_path.clone()
        } else {
            state.get_active_panel().current_path.clone()
        };
        state.dialogs.replace(CompressPrompt {
            input: default_name.into(),
            targets,
            dest_dir,
        });
    }
    true
}
