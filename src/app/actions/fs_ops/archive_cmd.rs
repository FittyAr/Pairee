use crate::app::state::AppState;
use crate::app::state::popup::ArchiveCommandsMenu;
use crate::config::localization::t;

pub fn handle(state: &mut AppState) -> bool {
    if let Some(archive_path) = super::extract::cursor_file(state) {
        state.dialogs.replace(ArchiveCommandsMenu {
            archive_path,
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
