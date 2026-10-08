use crate::app::context::AppContext;
use crate::app::state::{AppState, DialogStack, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::submit_simple;
use std::path::PathBuf;

fn is_non_empty_dir(path: &std::path::Path) -> bool {
    if path.is_dir() {
        if let Ok(mut entries) = std::fs::read_dir(path) {
            entries.next().is_some()
        } else {
            false
        }
    } else {
        false
    }
}

/// F8: deletes the targeted items of the active panel.
pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    request(state, context, targets, false);
    true
}

/// Deletes `targets` (items of the active panel's filesystem), asking first
/// when the confirmation settings say so. With `over_dialog` the confirmation
/// opens on top of the current dialog, which comes back afterwards.
pub fn request(
    state: &mut AppState,
    context: &AppContext,
    targets: Vec<PathBuf>,
    over_dialog: bool,
) {
    if targets.is_empty() {
        return;
    }
    let active_panel = state.get_active_panel();
    let is_remote = !active_panel.source.is_local();
    let show_prompt = context.config.settings.confirmations.confirm_delete
        || (context
            .config
            .settings
            .confirmations
            .confirm_delete_non_empty_folders
            && targets.iter().any(|p| {
                if is_remote {
                    active_panel
                        .entries
                        .iter()
                        .any(|e| &e.path == p && e.is_dir)
                } else {
                    is_non_empty_dir(p)
                }
            }));

    if show_prompt {
        let confirm = PopupType::ConfirmDelete {
            paths: targets,
            cursor_idx: 0,
        };
        if over_dialog {
            state.dialogs.push(confirm);
        } else {
            state.dialogs.replace(confirm);
        }
    } else {
        let ssh = state.get_active_panel().source.ssh().cloned();
        let options = TransferOptions {
            delete_to_recycle_bin: context.config.settings.delete_to_recycle_bin,
            ..Default::default()
        };
        submit_keeping_dialogs(state, TransferOperation::Delete, targets, options, ssh);
        state.get_active_panel_mut().clear_selection();
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
}

/// Queues a delete or wipe job. Submitting a job closes every dialog; the
/// ones that were open (e.g. the disk usage view) are put back afterwards.
pub fn submit_keeping_dialogs(
    state: &mut AppState,
    operation: TransferOperation,
    paths: Vec<PathBuf>,
    options: TransferOptions,
    ssh: Option<crate::fs::ssh::SharedSshClient>,
) {
    let open = std::mem::replace(&mut state.dialogs, DialogStack::new());
    submit_simple(state, operation, paths, PathBuf::new(), options, ssh, None);
    if !open.is_empty() {
        state.dialogs = open;
    }
}
