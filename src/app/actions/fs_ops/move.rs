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

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if targets.is_empty() {
        return true;
    }

    let dest_dir = state.get_passive_panel().current_path.clone();

    if !context.config.settings.confirmations.confirm_move {
        submit_move_job(state, context, targets, dest_dir, None);
        return true;
    }

    let default_input = if targets.len() == 1 {
        targets
            .first()
            .and_then(|p| p.file_name())
            .map(|n| dest_dir.join(n).to_string_lossy().to_string())
            .unwrap_or_else(|| dest_dir.to_string_lossy().to_string())
    } else {
        dest_dir.to_string_lossy().to_string()
    };

    state.active_popup = Some(PopupType::MovePrompt {
        input: default_input,
        src_paths: targets,
        dest_dir,
        cursor_idx: 0,
        already_existing: 0,
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
    true
}

/// Build and dispatch a Move transfer job using the options stored on the popup state.
/// `src_paths`/`dest` and the per-popup options are passed in; the
/// active/passive panel endpoints are derived from the panel state
/// at the moment of dispatch.
pub fn submit_move_job_from_popup(
    state: &mut AppState,
    context: &mut AppContext,
    src_paths: Vec<std::path::PathBuf>,
    input: String,
    already_existing: usize,
    copy_extended_attributes: bool,
    disable_write_cache: bool,
    symlink_mode: usize,
    use_filter: bool,
    filter_mask: String,
) {
    let dest_dir = state.get_passive_panel().current_path.clone();
    let dest = if input.trim().is_empty() {
        dest_dir.clone()
    } else {
        let candidate = std::path::PathBuf::from(&input);
        if candidate.is_absolute() {
            candidate
        } else {
            dest_dir.join(&input)
        }
    };

    let popup_opts = PopupOptions {
        already_existing,
        copy_extended_attributes,
        disable_write_cache,
        symlink_mode,
        use_filter,
        filter_mask,
    };
    submit_move_job(state, context, src_paths, dest, Some(popup_opts));
}

#[derive(Clone)]
struct PopupOptions {
    already_existing: usize,
    copy_extended_attributes: bool,
    disable_write_cache: bool,
    symlink_mode: usize,
    use_filter: bool,
    filter_mask: String,
}

fn submit_move_job(
    state: &mut AppState,
    context: &mut AppContext,
    targets: Vec<std::path::PathBuf>,
    dest: std::path::PathBuf,
    popup_opts: Option<PopupOptions>,
) {
    use crate::fs::transfer::engine::TransferEngine;
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
    options.direct_io = popup_opts
        .as_ref()
        .map(|p| p.disable_write_cache)
        .unwrap_or(false);
    options.preserve_timestamps = context.config.settings.transfer_preserve_timestamps;
    options.preserve_attributes = popup_opts
        .as_ref()
        .map(|p| p.copy_extended_attributes)
        .unwrap_or(false);
    options.preserve_acl = context.config.settings.transfer_preserve_acl;
    options.preserve_streams = context.config.settings.transfer_preserve_streams;
    options.limit_bandwidth_rate = context.config.settings.transfer_limit_bandwidth_rate;
    options.halt_on_error = context.config.settings.transfer_halt_on_error;
    options.max_retries = context.config.settings.transfer_max_retries;
    options.conflict_resolution = match popup_opts.as_ref().map(|p| p.already_existing).unwrap_or(0)
    {
        1 => "overwrite".to_string(),
        2 => "skip".to_string(),
        3 => "overwrite_older".to_string(),
        4 => "rename".to_string(),
        _ => "ask".to_string(),
    };
    match popup_opts.as_ref().map(|p| p.symlink_mode).unwrap_or(0) {
        1 => {
            options.skip_symlinks = false;
            options.follow_symlinks = true;
        }
        2 => {
            options.skip_symlinks = true;
            options.follow_symlinks = false;
        }
        _ => {
            options.skip_symlinks = false;
            options.follow_symlinks = false;
        }
    }
    options.filter_mask = popup_opts
        .as_ref()
        .filter(|p| p.use_filter && !p.filter_mask.is_empty())
        .map(|p| p.filter_mask.clone());

    let src_endpoint = endpoint_for_panel(state.get_active_panel());
    let dst_endpoint = endpoint_for_panel(state.get_passive_panel());

    let job = TransferJob::with_endpoints(
        TransferOperation::Move,
        targets,
        dest,
        options,
        src_endpoint,
        dst_endpoint,
    );

    for src in &job.sources {
        crate::fs::transfer::history::add_source_path(src);
    }
    crate::fs::transfer::history::add_dest_path(&job.destination);

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
