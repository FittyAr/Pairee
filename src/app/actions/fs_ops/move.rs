use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::transfer::engine::TransferEngine;
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if targets.is_empty() {
        return true;
    }

    let dest_dir = state.get_passive_panel().current_path.clone();

    if !context.config.settings.confirmations.confirm_move {
        submit_move_job(state, context, targets, dest_dir);
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
/// Extracts the heavy lifting so it can be shared with the no-confirm code path.
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

    submit_move_job_inner(
        state,
        context,
        src_paths,
        dest,
        already_existing,
        copy_extended_attributes,
        disable_write_cache,
        symlink_mode,
        use_filter,
        filter_mask,
    );
}

fn submit_move_job(
    state: &mut AppState,
    context: &mut AppContext,
    targets: Vec<std::path::PathBuf>,
    dest_dir: std::path::PathBuf,
) {
    submit_move_job_inner(
        state,
        context,
        targets,
        dest_dir,
        0,
        false,
        false,
        0,
        false,
        String::new(),
    );
}

fn submit_move_job_inner(
    state: &mut AppState,
    context: &mut AppContext,
    targets: Vec<std::path::PathBuf>,
    dest: std::path::PathBuf,
    already_existing: usize,
    copy_extended_attributes: bool,
    disable_write_cache: bool,
    symlink_mode: usize,
    use_filter: bool,
    filter_mask: String,
) {
    let is_ssh =
        state.get_active_panel().ssh_conn.is_some() || state.get_passive_panel().ssh_conn.is_some();

    if is_ssh {
        let rx = crate::fs::spawn_copy_move_task(
            targets.clone(),
            dest.clone(),
            state.get_active_panel().ssh_conn.clone(),
            state.get_passive_panel().ssh_conn.clone(),
            true,
            context.config.settings.clone(),
        );
        state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Move);
        state.progress_rx = Some(rx);
        state.active_popup = Some(PopupType::CopyProgress {
            is_move: true,
            current_file: t("progress_initializing"),
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
        });
        return;
    }

    let mut options = TransferOptions::default();
    options.verify_after_copy = context.config.settings.transfer_verify_after_copy;
    options.hash_algorithm = match context.config.settings.transfer_default_hash.as_str() {
        "crc32" => crate::fs::transfer::options::HashAlgorithm::Crc32,
        "md5" => crate::fs::transfer::options::HashAlgorithm::Md5,
        "sha1" => crate::fs::transfer::options::HashAlgorithm::Sha1,
        "sha256" => crate::fs::transfer::options::HashAlgorithm::Sha256,
        _ => crate::fs::transfer::options::HashAlgorithm::Blake3,
    };
    options.buffer_size = match context.config.settings.transfer_buffer_size {
        65536 => crate::fs::transfer::options::BufferSize::_64KB,
        262144 => crate::fs::transfer::options::BufferSize::_256KB,
        4194304 => crate::fs::transfer::options::BufferSize::_4MB,
        _ => crate::fs::transfer::options::BufferSize::_1MB,
    };
    options.direct_io = disable_write_cache;
    options.preserve_timestamps = context.config.settings.transfer_preserve_timestamps;
    options.preserve_attributes = copy_extended_attributes;
    options.preserve_acl = context.config.settings.transfer_preserve_acl;
    options.preserve_streams = context.config.settings.transfer_preserve_streams;
    options.limit_bandwidth_rate = context.config.settings.transfer_limit_bandwidth_rate;
    options.halt_on_error = context.config.settings.transfer_halt_on_error;
    options.max_retries = context.config.settings.transfer_max_retries;
    options.conflict_resolution = match already_existing {
        1 => "overwrite".to_string(),
        2 => "skip".to_string(),
        3 => "overwrite_older".to_string(),
        4 => "rename".to_string(),
        _ => "ask".to_string(),
    };
    match symlink_mode {
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
    options.filter_mask = if use_filter && !filter_mask.is_empty() {
        Some(filter_mask)
    } else {
        None
    };

    let job = TransferJob::new(TransferOperation::Move, targets, dest, options);

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
