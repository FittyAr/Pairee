use crate::app::context::AppContext;
use crate::app::state::panel::PanelState;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::endpoint::TransferEndpoint;

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

fn endpoint_for_panel(panel: &PanelState) -> TransferEndpoint {
    match &panel.ssh_conn {
        Some(client) => TransferEndpoint::Ssh(client.clone()),
        None => TransferEndpoint::Local,
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
            // Phase 6: always go through the unified engine. The
            // active panel's endpoint (Local or Ssh) becomes the
            // engine's source endpoint; deletes don't need a
            // separate destination endpoint so we pass the same
            // one.
            use crate::fs::transfer::engine::TransferEngine;
            use crate::fs::transfer::job::{TransferJob, TransferOperation};
            use crate::fs::transfer::options::TransferOptions;

            let mut options = TransferOptions::default();
            options.delete_to_recycle_bin = context.config.settings.delete_to_recycle_bin;

            let endpoint = endpoint_for_panel(state.get_active_panel());

            let job = TransferJob::with_endpoints(
                TransferOperation::Delete,
                targets.clone(),
                std::path::PathBuf::new(),
                options,
                endpoint.clone(),
                endpoint,
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
    true
}
