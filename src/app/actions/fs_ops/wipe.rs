use crate::app::context::AppContext;
use crate::app::state::panel::PanelState;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::endpoint::TransferEndpoint;

fn endpoint_for_panel(panel: &PanelState) -> TransferEndpoint {
    match &panel.ssh_conn {
        Some(client) => TransferEndpoint::Ssh(client.clone()),
        None => TransferEndpoint::Local,
    }
}

pub fn handle(state: &mut AppState, _context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        // Phase 9: secure wipe is now a `Delete` job with
        // `wipe_passes = 3` routed through the unified engine.
        // The WipeConfirm popup still exists for the
        // confirmation step, but the work itself is the same
        // engine path as Delete.
        state.active_popup = Some(PopupType::WipeConfirm { paths: targets });
    }
    true
}

/// Dispatch a confirmed wipe. The number of passes is hard-coded
/// to 3; the engine clamps the configured value to that range.
pub fn commit(state: &mut AppState, paths: Vec<std::path::PathBuf>) {
    use crate::fs::transfer::engine::TransferEngine;
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use crate::fs::transfer::options::TransferOptions;

    let endpoint = endpoint_for_panel(state.get_active_panel());
    let mut options = TransferOptions::default();
    options.wipe_passes = 3;

    let job = TransferJob::with_endpoints(
        TransferOperation::Delete,
        paths,
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
    state.active_popup = None;
}
