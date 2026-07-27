use crate::app::context::AppContext;
use crate::app::state::panel::PanelState;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::endpoint::TransferEndpoint;

/// Build a `TransferEndpoint` from a `PanelState` — `Local` if the
/// panel has no SSH connection, otherwise the Ssh wrapper.
fn endpoint_for_panel(panel: &PanelState) -> TransferEndpoint {
    match &panel.ssh_conn {
        Some(client) => TransferEndpoint::Ssh(client.clone()),
        None => TransferEndpoint::Local,
    }
}

/// Build a fully-configured `TransferJob::Copy` with the per-panel
/// endpoints and the user-level transfer options applied. Used by
/// both the no-confirmation `handle` path and the `CopyPrompt`
/// `Enter` handler so the two stay in lockstep.
pub fn submit_copy_job(
    state: &mut AppState,
    context: &AppContext,
    sources: Vec<std::path::PathBuf>,
    destination: std::path::PathBuf,
) {
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use crate::fs::transfer::options::{BufferSize, HashAlgorithm, TransferOptions};

    let mut options = TransferOptions::default();
    options.verify_after_copy = context.config.settings.transfer_verify_after_copy;
    options.hash_algorithm = match context.config.settings.transfer_default_hash.as_str() {
        "crc32" => HashAlgorithm::Crc32,
        "md5" => HashAlgorithm::Md5,
        "sha1" => HashAlgorithm::Sha1,
        "sha256" => HashAlgorithm::Sha256,
        _ => HashAlgorithm::Blake3,
    };
    options.buffer_size = match context.config.settings.transfer_buffer_size {
        65536 => BufferSize::_64KB,
        262144 => BufferSize::_256KB,
        4194304 => BufferSize::_4MB,
        _ => BufferSize::_1MB,
    };
    options.direct_io = context.config.settings.transfer_direct_io;
    options.preserve_timestamps = context.config.settings.transfer_preserve_timestamps;
    options.preserve_attributes = context.config.settings.transfer_preserve_attributes;
    options.preserve_acl = context.config.settings.transfer_preserve_acl;
    options.preserve_streams = context.config.settings.transfer_preserve_streams;
    options.skip_symlinks = context.config.settings.transfer_skip_symlinks;
    options.follow_symlinks = context.config.settings.transfer_follow_symlinks;
    options.limit_bandwidth_rate = context.config.settings.transfer_limit_bandwidth_rate;
    options.halt_on_error = context.config.settings.transfer_halt_on_error;
    options.max_retries = context.config.settings.transfer_max_retries;
    options.conflict_resolution = context.config.settings.transfer_conflict_resolution.clone();

    let src_endpoint = endpoint_for_panel(state.get_active_panel());
    let dst_endpoint = endpoint_for_panel(state.get_passive_panel());

    let job = TransferJob::with_endpoints(
        TransferOperation::Copy,
        sources,
        destination,
        options,
        src_endpoint,
        dst_endpoint,
    );

    if state.transfer.is_none() {
        let (engine, rx) = crate::fs::transfer::engine::TransferEngine::new();
        state.transfer = Some(crate::app::state::transfer_state::TransferUIState::new(
            engine, rx,
        ));
    }
    if let Some(ref mut ts) = state.transfer {
        ts.engine.submit_job(job);
        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
    }
}

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let dest_dir = state.get_passive_panel().current_path.clone();
        if context.config.settings.confirmations.confirm_copy {
            let default_input = if targets.len() == 1 {
                targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| dest_dir.join(n).to_string_lossy().to_string())
                    .unwrap_or_else(|| dest_dir.to_string_lossy().to_string())
            } else {
                dest_dir.to_string_lossy().to_string()
            };
            state.active_popup = Some(PopupType::CopyPrompt {
                input: default_input,
                src_paths: targets,
                dest_dir,
                cursor_idx: 0,
                already_existing: 0, // Ask
                process_multiple: false,
                copy_access_mode: true,
                copy_extended_attributes: false,
                disable_write_cache: false,
                produce_sparse_files: false,
                use_copy_on_write: false,
                symlink_mode: 0,
                use_filter: false,
                filter_mask: String::new(),
            });
        } else {
            // Phase 5: the transfer engine is now endpoint-aware
            // and handles both local and SSH transparently. The
            // legacy `spawn_copy_move_task` modal is no longer
            // needed for Copy.
            submit_copy_job(state, context, targets, dest_dir);
            state.active_popup = None;
        }
    }
    true
}
