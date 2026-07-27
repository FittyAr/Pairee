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
            // Format detection: use the file
            // extension. The engine requires an
            // explicit ArchiveFormat; we map
            // .zip / .tar.gz / .7z here. Other
            // extensions fall back to Zip (matches
            // the legacy default).
            let format = match archive
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("zip") => ArchiveFormat::Zip,
                Some("gz") | Some("tgz") => ArchiveFormat::TarGz,
                Some("7z") => ArchiveFormat::SevenZ,
                _ => ArchiveFormat::Zip,
            };
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
