use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let dest = state.get_passive_panel().current_path.clone();
        let rx = crate::fs::spawn_extract_task(entry.path.clone(), dest);
        state.progress_rx = Some(rx);
        state.active_popup = Some(PopupType::CopyProgress {
            is_move: false,
            current_file: t("progress_extracting"),
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
        });
    }
    true
}
