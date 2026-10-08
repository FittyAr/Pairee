//! Actions of the transfer dialog buttons: submit, folder tree, filter.

use crate::app::context::AppContext;
use crate::app::state::types::TreeViewCaller;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::{submit_simple, transfer_options_from_settings};

/// Closes the dialog and queues the Copy / Move job it describes.
pub fn submit(state: &mut AppState, context: &AppContext) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.pop() else {
        return;
    };
    state.dialogs.clear();
    let mut options = transfer_options_from_settings(&context.config.settings);
    prompt.apply_to(&mut options);
    let destination = prompt.destination();
    submit_simple(
        state,
        prompt.op.operation(),
        prompt.src_paths,
        destination,
        options,
        state.get_active_panel().ssh_conn.clone(),
        state.get_passive_panel().ssh_conn.clone(),
    );
}

/// F10 / "Tree" button: pick the destination from a folder tree.
pub fn open_tree_view(state: &mut AppState) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let nodes = crate::app::sys_helpers::build_tree_nodes(&prompt.dest_dir, 0, 3);
    state.dialogs.open_over(|previous| PopupType::TreeView {
        nodes,
        cursor_idx: 0,
        caller: TreeViewCaller::TransferPrompt { previous },
    });
}

/// "Filter" button: edit the file mask applied to the transfer.
pub fn open_filter_prompt(state: &mut AppState) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let input = prompt.filter_mask.clone();
    state
        .dialogs
        .open_over(|previous| PopupType::CopyMoveFilterPrompt { input, previous });
}
