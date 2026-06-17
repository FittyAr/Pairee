use crate::app::state::{AppState, LinkKind, PopupType};

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        if entry.name != ".." {
            state.active_popup = Some(PopupType::CreateLinkPrompt {
                src: entry.path.clone(),
                dest_input: entry.name.clone(),
                kind: LinkKind::Symbolic,
            });
        }
    }
    true
}
