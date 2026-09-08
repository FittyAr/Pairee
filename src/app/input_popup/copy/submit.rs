use crate::app::context::AppContext;
use crate::app::state::AppState;
use std::path::PathBuf;

pub fn submit_copy_job(
    state: &mut AppState,
    context: &AppContext,
    src_paths: Vec<PathBuf>,
    dest_dir: &std::path::Path,
    new_input: &str,
    new_already: usize,
    new_ext: bool,
    new_cache: bool,
    new_sym: usize,
    new_filter: bool,
    new_filter_mask: &str,
) {
    let targets = src_paths;
    let dest = dest_dir.join(new_input);

    let mut options = crate::fs::transfer::transfer_options_from_settings(&context.config.settings);
    options.direct_io = new_cache;
    options.preserve_attributes = new_ext;
    options.conflict_resolution = match new_already {
        1 => "overwrite".to_string(),
        2 => "skip".to_string(),
        3 => "overwrite_older".to_string(),
        4 => "rename".to_string(),
        _ => "ask".to_string(),
    };
    match new_sym {
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
    options.filter_mask = if new_filter && !new_filter_mask.is_empty() {
        Some(new_filter_mask.to_string())
    } else {
        None
    };

    crate::fs::transfer::submit_simple(
        state,
        crate::fs::transfer::job::TransferOperation::Copy,
        targets,
        dest,
        options,
        state.get_active_panel().ssh_conn.clone(),
        state.get_passive_panel().ssh_conn.clone(),
    );
}

pub fn toggle_option(
    idx: usize,
    already: &mut usize,
    multi: &mut bool,
    access: &mut bool,
    ext: &mut bool,
    cache: &mut bool,
    sparse: &mut bool,
    cow: &mut bool,
    sym: &mut usize,
    filter: &mut bool,
) {
    match idx {
        1 => *already = (*already + 1) % 4,
        2 => *multi = !*multi,
        3 => *access = !*access,
        4 => *ext = !*ext,
        5 => *cache = !*cache,
        6 => *sparse = !*sparse,
        7 => *cow = !*cow,
        8 => *sym = (*sym + 1) % 3,
        9 => *filter = !*filter,
        _ => {}
    }
}

pub fn move_cursor_vertical(idx: usize, up: bool, max_idx: usize) -> usize {
    if up {
        if idx > 0 { idx - 1 } else { max_idx }
    } else if idx < max_idx {
        idx + 1
    } else {
        0
    }
}

pub fn move_horizontal(idx: usize, left: bool) -> usize {
    if (10..=13).contains(&idx) {
        if left {
            if idx > 10 { idx - 1 } else { 13 }
        } else if idx < 13 {
            idx + 1
        } else {
            10
        }
    } else {
        idx
    }
}

pub fn open_tree_view(state: &mut AppState, dest_dir: &std::path::Path) {
    let nodes = crate::app::sys_helpers::build_tree_nodes(dest_dir, 0, 3);
    let previous = Box::new(state.dialogs.take().unwrap());
    state
        .dialogs
        .replace(crate::app::state::PopupType::TreeView {
            nodes,
            cursor_idx: 0,
            caller: crate::app::state::types::TreeViewCaller::CopyPrompt { previous },
        });
}

pub fn autocomplete_destination(input: &str) -> Option<String> {
    if input.is_empty() {
        return None;
    }
    let history = crate::fs::transfer::history::load_history();
    history
        .destinations
        .into_iter()
        .find(|d| d.to_lowercase().starts_with(&input.to_lowercase()))
}
