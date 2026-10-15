//! Actions of the transfer dialog buttons: submit, folder tree, filter.

use crate::app::context::AppContext;
use crate::app::state::popup::CopyMoveFilterPrompt;
use crate::app::state::types::TreeViewCaller;
use crate::app::state::{AppState, PopupType};
use crate::app::transfer_submit::submit_job;
use crate::fs::transfer::job::TransferJob;
use crate::fs::transfer::transfer_options_from_settings;

/// Closes the dialog and queues the Copy / Move job it describes.
pub fn submit(state: &mut AppState, context: &AppContext) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.pop() else {
        return;
    };
    state.dialogs.clear();
    let mut options = transfer_options_from_settings(&context.config.settings);
    prompt.apply_to(&mut options);
    let (src_ssh, dst_ssh) = prompt.endpoints.clone().unwrap_or_else(|| {
        (
            state.get_active_panel().source.remote().cloned(),
            state.get_passive_panel().source.remote().cloned(),
        )
    });
    // Several destinations become several jobs, run in this order.
    for (operation, destination) in prompt.jobs() {
        let folder = destination == prompt.dest_dir;
        let job = TransferJob::new(
            operation,
            prompt.src_paths.clone(),
            destination,
            options.clone(),
        )
        .with_remotes(src_ssh.clone(), dst_ssh.clone());
        submit_job(state, if folder { job.into_folder() } else { job });
    }
}

/// F10 / "Tree" button: pick the destination from a folder tree.
pub fn open_tree_view(state: &mut AppState) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let root = prompt.dest_dir.clone();
    crate::app::dialog_loads::open_tree(state, root, TreeViewCaller::TransferPrompt, true);
}

/// "Filter" button: edit the file mask applied to the transfer.
pub fn open_filter_prompt(state: &mut AppState) {
    let Some(PopupType::TransferPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let input = prompt.filter_mask.as_str().into();
    state.dialogs.push(CopyMoveFilterPrompt { input });
}
