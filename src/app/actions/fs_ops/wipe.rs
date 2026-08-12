use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::submit_simple;

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        if context.config.settings.confirmations.confirm_wipe {
            state.active_popup = Some(PopupType::WipeConfirm { paths: targets });
        } else {
            submit_simple(
                state,
                TransferOperation::Wipe,
                targets,
                std::path::PathBuf::new(),
                TransferOptions::default(),
                None,
                None,
            );
            state.get_active_panel_mut().clear_selection();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
    }
    true
}
