use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::{submit_simple, transfer_options_from_settings};

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

    state.dialogs.replace(PopupType::MovePrompt {
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
    let mut options = transfer_options_from_settings(&context.config.settings);
    options.direct_io = disable_write_cache;
    options.preserve_attributes = copy_extended_attributes;
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

    submit_simple(
        state,
        TransferOperation::Move,
        targets,
        dest,
        options,
        state.get_active_panel().ssh_conn.clone(),
        state.get_passive_panel().ssh_conn.clone(),
    );
}
