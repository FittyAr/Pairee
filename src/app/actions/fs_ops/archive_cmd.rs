use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn handle(
    state: &mut AppState,
) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        state.active_popup = Some(PopupType::ArchiveCommandsMenu {
            archive_path: entry.path.clone(),
            items: vec![
                t("menu_archive_list"),
                t("menu_archive_test"),
                t("menu_archive_extract"),
                t("menu_archive_extract_other"),
            ],
            cursor_idx: 0,
        });
    }
    true
}
