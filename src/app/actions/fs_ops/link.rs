use crate::app::state::panel::PanelState;
use crate::app::state::{AppState, LinkKind, PopupType};
use crate::fs::transfer::endpoint::TransferEndpoint;

fn endpoint_for_panel(panel: &PanelState) -> TransferEndpoint {
    match &panel.ssh_conn {
        Some(client) => TransferEndpoint::Ssh(client.clone()),
        None => TransferEndpoint::Local,
    }
}

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        if entry.name != ".." {
            state.active_popup = Some(PopupType::CreateLinkPrompt {
                src: entry.path.clone(),
                dest_input: entry.name.clone(),
                kind: LinkKind::Symbolic,
            });
        }
    }
    true
}

/// Dispatch the create-link popup. Phase 8: the link goes
/// through the unified engine instead of `fs::create_symlink`
/// or `fs::create_hardlink` directly. Both endpoints equal the
/// passive panel (the link's destination lives there).
pub fn commit(
    state: &mut AppState,
    src: std::path::PathBuf,
    dest: std::path::PathBuf,
    kind: LinkKind,
) {
    use crate::fs::transfer::engine::TransferEngine;
    use crate::fs::transfer::job::{LinkKind as EngineLinkKind, TransferJob, TransferOperation};
    use crate::fs::transfer::options::TransferOptions;

    let endpoint = endpoint_for_panel(state.get_passive_panel());
    let options = TransferOptions::default();

    let engine_kind = match kind {
        LinkKind::Symbolic => EngineLinkKind::Symbolic,
        LinkKind::Hard => EngineLinkKind::Hard,
    };

    let job = TransferJob::with_endpoints(
        TransferOperation::CreateLink { kind: engine_kind },
        vec![src],
        dest,
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
