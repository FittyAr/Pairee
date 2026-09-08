use crate::app::actions::fs_ops::r#move as move_action;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use std::path::PathBuf;

pub fn submit_move_job(
    state: &mut AppState,
    context: &mut AppContext,
    src_paths: Vec<PathBuf>,
    new_input: String,
    new_already: usize,
    new_ext: bool,
    new_cache: bool,
    new_sym: usize,
    new_filter: bool,
    new_filter_mask: String,
) {
    move_action::submit_move_job_from_popup(
        state,
        context,
        src_paths,
        new_input,
        new_already,
        new_ext,
        new_cache,
        new_sym,
        new_filter,
        new_filter_mask,
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
            caller: crate::app::state::types::TreeViewCaller::MovePrompt { previous },
        });
}
