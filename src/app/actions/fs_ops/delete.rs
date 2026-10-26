use crate::app::context::AppContext;
use crate::app::state::popup::{ConfirmDelete, ConfirmRetryAsAdmin};
use crate::app::state::{AdminOpKind, AppState, DialogStack, PopupType};
use crate::app::transfer_submit::submit_simple;
use crate::config::localization::t;
use crate::fs::transfer::job::{FailedFile, TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;
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

/// Where deleted items go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    /// As the "Delete to Recycle Bin" setting says (`delete`, F8).
    Configured,
    /// Always the recycle bin (`trash`).
    Trash,
    /// Never the recycle bin (`delete_permanent`).
    Permanent,
}

impl DeleteMode {
    fn to_trash(self, context: &AppContext) -> bool {
        match self {
            Self::Configured => context.config.settings.delete_to_recycle_bin,
            Self::Trash => true,
            Self::Permanent => false,
        }
    }
}

/// Deletes the targeted items of the active panel.
pub fn handle(state: &mut AppState, context: &mut AppContext, mode: DeleteMode) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    request(state, context, targets, false, mode);
    true
}

/// Whether `path` is a folder (not a link to one): from the panel's
/// listing, else (local paths listed elsewhere) from the disk.
fn is_folder(panel: &crate::app::state::PanelState, path: &std::path::Path, remote: bool) -> bool {
    match panel.entries.iter().find(|e| e.path == path) {
        Some(entry) => entry.is_dir && !entry.is_symlink,
        None if remote => false,
        None => std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir()),
    }
}

/// Deletes `targets` (items of the active panel's filesystem), asking first
/// when the confirmation settings say so. With `over_dialog` the confirmation
/// opens on top of the current dialog, which comes back afterwards.
///
/// A non-local source has no recycle bin: an explicit "trash" is refused,
/// and "Delete to Recycle Bin" does not apply there (the question says the
/// deletion is permanent).
pub fn request(
    state: &mut AppState,
    context: &AppContext,
    targets: Vec<PathBuf>,
    over_dialog: bool,
    mode: DeleteMode,
) {
    if targets.is_empty() {
        return;
    }
    let active_panel = state.get_active_panel();
    let is_remote = !active_panel.source.is_local();
    if is_remote && mode == DeleteMode::Trash {
        let message = crate::config::localization::t("trash_unsupported");
        if over_dialog {
            state.dialogs.push(PopupType::info(message));
        } else {
            state.dialogs.replace(PopupType::info(message));
        }
        return;
    }
    let to_trash = mode.to_trash(context) && !is_remote;
    let show_prompt = context.config.settings.confirmations.confirm_delete
        || (context
            .config
            .settings
            .confirmations
            .confirm_delete_non_empty_folders
            && targets.iter().any(|p| {
                if is_remote {
                    // Not listed (disk usage, duplicates): may be a folder.
                    !active_panel
                        .entries
                        .iter()
                        .any(|e| &e.path == p && !e.is_dir)
                } else {
                    is_non_empty_dir(p)
                }
            }));

    if show_prompt {
        let folders = targets
            .iter()
            .filter(|p| is_folder(active_panel, p, is_remote))
            .count();
        let confirm = PopupType::ConfirmDelete(ConfirmDelete {
            paths: targets,
            cursor_idx: 0,
            to_trash,
            folders,
        });
        if over_dialog {
            state.dialogs.push(confirm);
        } else {
            state.dialogs.replace(confirm);
        }
    } else {
        submit_delete(state, context, targets, to_trash);
    }
}

/// Queues the deletion of `targets` from the active panel's source, then
/// clears its selection and rereads the panels.
pub fn submit_delete(
    state: &mut AppState,
    context: &AppContext,
    targets: Vec<PathBuf>,
    to_trash: bool,
) {
    let remote = state.get_active_panel().source.remote().cloned();
    let options = TransferOptions {
        delete_to_recycle_bin: to_trash,
        ..Default::default()
    };
    submit_keeping_dialogs(state, TransferOperation::Delete, targets, options, remote);
    after_delete_submitted(state, context);
}

/// Clears the active panel's selection and rereads both panels.
pub fn after_delete_submitted(state: &mut AppState, context: &AppContext) {
    state.get_active_panel_mut().clear_selection();
    state.refresh_both_panels(context.config.settings.show_hidden);
}

/// Queues a delete or wipe job. Submitting a job closes every dialog; the
/// ones that were open (e.g. the disk usage view) are put back afterwards.
pub fn submit_keeping_dialogs(
    state: &mut AppState,
    operation: TransferOperation,
    paths: Vec<PathBuf>,
    options: TransferOptions,
    remote: Option<crate::fs::remote::RemoteConn>,
) {
    let open = std::mem::replace(&mut state.dialogs, DialogStack::new());
    submit_simple(
        state,
        operation,
        paths,
        PathBuf::new(),
        options,
        remote,
        None,
    );
    if !open.is_empty() {
        state.dialogs = open;
    }
}

/// What the user sees when `job` ends with `failed` items: the count, the
/// first refused path and its reason. A permanent local delete that the
/// system refused for lack of permission offers to delete what is left
/// with administrator rights (unless Pairee already has them); everything
/// else shows the reason.
pub fn failure_popup(job: &TransferJob, failed: &[FailedFile]) -> Option<PopupType> {
    let message = failure_message(job.operation, failed)?;
    if !may_retry_as_admin(job, failed) {
        return Some(PopupType::error(message));
    }
    // The sources still there: the elevated helper deletes each whole.
    let paths: Vec<PathBuf> = job
        .sources
        .iter()
        .filter(|p| crate::fs::stamp::entry_exists(p))
        .cloned()
        .collect();
    if paths.is_empty() {
        return Some(PopupType::error(message));
    }
    Some(PopupType::ConfirmRetryAsAdmin(ConfirmRetryAsAdmin {
        paths,
        op_kind: AdminOpKind::Delete { reason: message },
    }))
}

/// "N items could not be deleted (transferred). First: path: reason",
/// naming the first refused permission when there is one.
fn failure_message(operation: TransferOperation, failed: &[FailedFile]) -> Option<String> {
    let first = failed.iter().find(|f| f.denied).or(failed.first())?;
    let key = match operation {
        TransferOperation::Delete | TransferOperation::Wipe => "delete_items_failed",
        _ => "transfer_files_failed",
    };
    Some(
        t(key)
            .replacen("{}", &failed.len().to_string(), 1)
            .replacen("{}", &first.src.to_string_lossy(), 1)
            .replacen("{}", &first.error, 1),
    )
}

/// A permanent delete on the local disk that the system refused for lack
/// of permission, run without administrator rights.
fn may_retry_as_admin(job: &TransferJob, failed: &[FailedFile]) -> bool {
    job.operation == TransferOperation::Delete
        && !job.options.delete_to_recycle_bin
        && job.remote.is_none()
        && failed.iter().any(|f| f.denied)
        && !crate::fs::is_elevated()
}
