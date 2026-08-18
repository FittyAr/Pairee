use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::submit_simple;

fn is_non_empty_dir(path: &std::path::Path) -> bool {
    if path.is_dir() {
        if let Ok(mut entries) = std::fs::read_dir(path) {
            entries.next().is_some()
        } else {
            false
        }
    } else {
        false
    }
}

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let active_panel = state.get_active_panel();
        let is_remote = active_panel.ssh_conn.is_some();
        let show_prompt = context.config.settings.confirmations.confirm_delete
            || (context
                .config
                .settings
                .confirmations
                .confirm_delete_non_empty_folders
                && targets.iter().any(|p| {
                    if is_remote {
                        active_panel
                            .entries
                            .iter()
                            .any(|e| &e.path == p && e.is_dir)
                    } else {
                        is_non_empty_dir(p)
                    }
                }));

        if show_prompt {
            state.dialogs.replace(PopupType::ConfirmDelete {
                paths: targets,
                cursor_idx: 0,
            });
        } else {
            let ssh = state.get_active_panel().ssh_conn.clone();
            let options = TransferOptions {
                delete_to_recycle_bin: context.config.settings.delete_to_recycle_bin,
                ..Default::default()
            };
            submit_simple(
                state,
                TransferOperation::Delete,
                targets.clone(),
                std::path::PathBuf::new(),
                options,
                ssh,
                None,
            );
            state.get_active_panel_mut().clear_selection();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
    }
    true
}
