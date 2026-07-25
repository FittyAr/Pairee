use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

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
            state.active_popup = Some(PopupType::ConfirmDelete {
                paths: targets,
                cursor_idx: 0,
            });
        } else {
            let active_panel = state.get_active_panel();
            if let Some(client) = &active_panel.ssh_conn {
                let rx = crate::fs::spawn_ssh_delete_task(client.clone(), targets.clone());
                state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Delete);
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    is_move: false,
                    current_file: t("progress_initializing"),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                });
            } else {
                use crate::fs::transfer::engine::TransferEngine;
                use crate::fs::transfer::job::{TransferJob, TransferOperation};
                use crate::fs::transfer::options::TransferOptions;

                let mut options = TransferOptions::default();
                options.delete_to_recycle_bin = context.config.settings.delete_to_recycle_bin;

                let job = TransferJob::new(
                    TransferOperation::Delete,
                    targets.clone(),
                    std::path::PathBuf::new(),
                    options,
                );

                if state.transfer.is_none() {
                    let (engine, rx) = TransferEngine::new();
                    state.transfer = Some(crate::app::state::transfer_state::TransferUIState::new(
                        engine, rx,
                    ));
                }

                if let Some(ref mut ts) = state.transfer {
                    ts.engine.submit_job(job);
                    ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                }
                state.get_active_panel_mut().clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
    }
    true
}
