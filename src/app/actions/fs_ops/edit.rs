use crate::app::context::AppContext;
use crate::app::editor::open::open_in_editor;
use crate::app::state::AppState;

/// F4: opens the file under the cursor in the built-in editor.
pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let path = entry.path.clone();
        state.push_file_view_history(path.clone());
        open_in_editor(state, path, &context.config.settings);
    }
    true
}
