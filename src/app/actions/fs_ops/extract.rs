use crate::app::state::{AppState, PopupType};
use crate::app::transfer_submit::submit_simple;
use crate::config::localization::t;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use std::path::{Path, PathBuf};

/// Extract: the archive under the cursor goes into the other panel's folder.
pub fn handle(state: &mut AppState) -> bool {
    if let Some(archive) = cursor_file(state) {
        extract_to_other_panel(state, &archive);
    }
    true
}

/// The file (not a folder) under the active panel's cursor.
pub fn cursor_file(state: &AppState) -> Option<PathBuf> {
    let active = state.get_active_panel();
    active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
}

/// Extracts the local `archive` into the folder `dest` of the local disk.
pub fn extract_to(state: &mut AppState, archive: &Path, dest: PathBuf) {
    submit_simple(
        state,
        TransferOperation::Extract,
        vec![archive.to_path_buf()],
        dest,
        TransferOptions::default(),
        None,
        None,
    );
}

/// Extracts `archive` into the other panel's folder, which must be on the
/// local disk (extraction writes local files: a server path would be
/// created on the local disk instead).
pub fn extract_to_other_panel(state: &mut AppState, archive: &Path) {
    let passive = state.get_passive_panel();
    if !passive.source.is_local() {
        state
            .dialogs
            .replace(PopupType::info(t("vfs_action_unsupported")));
        return;
    }
    let dest = passive.current_path.clone();
    extract_to(state, archive, dest);
}
