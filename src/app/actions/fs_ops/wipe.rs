use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn handle(
    state: &mut AppState,
    context: &mut AppContext,
) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        if context.config.settings.confirmations.confirm_wipe {
            state.active_popup = Some(PopupType::WipeConfirm { paths: targets });
        } else {
            let rx = crate::fs::spawn_wipe_task(targets);
            state.progress_rx = Some(rx);
            state.active_popup = Some(PopupType::CopyProgress {
                is_move: false,
                current_file: t("progress_wiping"),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
            });
        }
    }
    true
}
