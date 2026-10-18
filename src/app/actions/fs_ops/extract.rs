use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::transfer_submit::submit_job;
use crate::config::localization::t;
use crate::fs::archive::is_browsable;
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::transfer_options_from_settings;
use std::path::{Path, PathBuf};

/// Extract: the selected archives (or the one under the cursor) go into
/// the other panel's folder.
pub fn handle(state: &mut AppState, context: &AppContext) -> bool {
    let archives: Vec<PathBuf> = state
        .get_active_panel()
        .get_targeted_paths()
        .into_iter()
        .filter(|path| is_archive_file(path))
        .collect();
    if archives.is_empty() {
        state
            .dialogs
            .replace(PopupType::info(t("archive_none_selected")));
        return true;
    }
    if let Some(dest) = other_panel_folder(state) {
        for archive in archives {
            extract_to(state, context, &archive, dest.clone());
        }
    }
    true
}

/// `path` is a file in a format that can be extracted.
fn is_archive_file(path: &Path) -> bool {
    is_browsable(path) && path.is_file()
}

/// The archive under the active panel's cursor, or `None` after saying
/// that it is not one.
pub fn cursor_archive(state: &mut AppState) -> Option<PathBuf> {
    let active = state.get_active_panel();
    let archive = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir && is_archive_file(&e.path))
        .map(|e| e.path.clone());
    if archive.is_none() {
        state
            .dialogs
            .replace(PopupType::info(t("archive_none_selected")));
    }
    archive
}

/// Extracts the local `archive` into the folder `dest` of the local disk;
/// files already there are resolved by the transfer conflict setting.
pub fn extract_to(state: &mut AppState, context: &AppContext, archive: &Path, dest: PathBuf) {
    let options = transfer_options_from_settings(&context.config.settings);
    let job = TransferJob::new(
        TransferOperation::Extract,
        vec![archive.to_path_buf()],
        dest,
        options,
    )
    .into_folder();
    submit_job(state, job);
}

/// The other panel's folder, which must be on the local disk (extraction
/// writes local files: a server path would be created on the local disk
/// instead); `None` after saying so.
fn other_panel_folder(state: &mut AppState) -> Option<PathBuf> {
    let passive = state.get_passive_panel();
    if !passive.source.is_local() {
        state
            .dialogs
            .replace(PopupType::info(t("vfs_action_unsupported")));
        return None;
    }
    Some(passive.current_path.clone())
}

/// Extracts `archive` into the other panel's folder.
pub fn extract_to_other_panel(state: &mut AppState, context: &AppContext, archive: &Path) {
    if let Some(dest) = other_panel_folder(state) {
        extract_to(state, context, archive, dest);
    }
}
