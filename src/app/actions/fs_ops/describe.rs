use crate::app::state::{AppState, PopupType};

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        if entry.name != ".." {
            let current_desc =
                crate::fs::read_description(&active.current_path.clone(), &entry.name)
                    .unwrap_or_default();
            state.active_popup = Some(PopupType::DescribeFilePrompt {
                path: entry.path.clone(),
                current_desc: current_desc.clone(),
                input: current_desc,
            });
        }
    }
    true
}
