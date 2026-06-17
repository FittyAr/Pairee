use crate::app::context::AppContext;
use crate::app::state::types::EditorState;
use crate::app::state::{AppState, Screen, PopupType};
use crate::config::localization::t;

pub fn handle(
    state: &mut AppState,
    _context: &mut AppContext,
) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let path = entry.path.clone();
        state.push_file_view_history(path.clone());
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                state.push_screen(Screen::Editor(EditorState {
                    path,
                    lines: if lines.is_empty() {
                        vec![String::new()]
                    } else {
                        lines
                    },
                    cursor_x: 0,
                    cursor_y: 0,
                    scroll_y: 0,
                    is_dirty: false,
                    last_search: None,
                    last_case_sensitive: false,
                }));
            }
            Err(e) => {
                state.active_popup = Some(PopupType::Error(format!(
                    "{} {}",
                    t("error_read_file_failed"),
                    e
                )));
            }
        }
    }
    true
}
