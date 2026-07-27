use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::engine::TransferEngine;
use crate::fs::transfer::job::{
    ArchiveFormat, TransferJob, TransferOperation,
};
use crate::fs::transfer::options::TransferOptions;
use crate::app::state::transfer_state::TransferUIState;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let archive = entry.path.clone();
        let dest = state.get_passive_panel().current_path.clone();

        // Format detection: central helper so the
        // rules stay in sync with
        // `input_popup/archive_commands.rs`. An
        // unrecognised extension used to silently fall
        // back to Zip, which produced a confusing
        // "not a zip file" error. Now we surface a
        // clear popup instead.
        let format = match ArchiveFormat::detect_from_path(&archive) {
            Some(f) => f,
            None => {
                state.active_popup = Some(PopupType::Error(format!(
                    "Unsupported archive format: {:?}. Supported: .zip, .tar.gz, .tgz, .tar, .7z",
                    archive.extension()
                )));
                return true;
            }
        };

        // A10: enqueue an Extract job on the unified
        // transfer engine. The legacy
        // `spawn_extract_task` and the
        // `state.progress_rx` channel are no longer
        // used for this path.
        if state.transfer.is_none() {
            let (engine, rx) = TransferEngine::new();
            state.transfer = Some(TransferUIState::new(engine, rx));
        }
        if let Some(ref mut ts) = state.transfer {
            let options = TransferOptions::default();
            let job = TransferJob::with_endpoints(
                TransferOperation::Extract { format },
                vec![archive],
                dest,
                options,
                crate::fs::transfer::endpoint::TransferEndpoint::Local,
                crate::fs::transfer::endpoint::TransferEndpoint::Local,
            );
            ts.engine.submit_job(job);
            ts.view_mode = crate::app::state::TransferViewMode::Minimized;
        }
        state.active_popup = Some(PopupType::TransferPanel);
    }
    true
}
