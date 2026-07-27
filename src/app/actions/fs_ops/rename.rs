use crate::app::context::AppContext;
use crate::app::state::panel::PanelState;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::transfer::endpoint::TransferEndpoint;

fn endpoint_for_panel(panel: &PanelState) -> TransferEndpoint {
    match &panel.ssh_conn {
        Some(client) => TransferEndpoint::Ssh(client.clone()),
        None => TransferEndpoint::Local,
    }
}

pub fn handle(state: &mut AppState, _context: &mut AppContext) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        let original = entry.name.clone();
        state.active_popup = Some(PopupType::RenamePrompt {
            input: original.clone(),
            original,
            src_path: entry.path.clone(),
            parent_dir: active.current_path.clone(),
            cursor_idx: 0,
        });
        true
    } else {
        state.active_popup = Some(PopupType::Error(t("error_no_entry_rename")));
        true
    }
}

/// Perform the actual rename on Enter. Phase 7: the rename goes
/// through the unified transfer engine, with both endpoints equal
/// to the active panel's endpoint (rename across panels is not
/// supported — use Move for that).
pub fn commit(
    state: &mut AppState,
    context: &mut AppContext,
    input: String,
    original: String,
    src_path: std::path::PathBuf,
    parent_dir: std::path::PathBuf,
) {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() || trimmed == original {
        state.active_popup = None;
        return;
    }
    let target = parent_dir.join(&trimmed);
    if target == src_path {
        state.active_popup = None;
        return;
    }

    use crate::fs::transfer::engine::TransferEngine;
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use crate::fs::transfer::options::TransferOptions;

    let endpoint = endpoint_for_panel(state.get_active_panel());
    let mut options = TransferOptions::default();
    options.conflict_resolution = context.config.settings.transfer_conflict_resolution.clone();
    options.halt_on_error = context.config.settings.transfer_halt_on_error;
    options.max_retries = context.config.settings.transfer_max_retries;

    let job = TransferJob::with_endpoints(
        TransferOperation::Rename,
        vec![src_path],
        target,
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
    state.refresh_both_panels(context.config.settings.show_hidden);
}
