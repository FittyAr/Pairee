//! Delivers a finished folder comparison on the UI thread.

use super::ScanPurpose;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::CompareStatus;
use crate::fs::sync::{SyncError, SyncItem, compare_entries};

/// Drains the folder-scan job; returns `true` when it finished this call.
pub fn poll_folder_scan(state: &mut AppState) -> bool {
    let Some(outcome) = state.folder_scan.job.poll() else {
        if state.folder_scan.is_running() {
            // Progress ticks: keep the counters moving.
            state.mark_ui_dirty();
        }
        return false;
    };
    let purpose = state.folder_scan.purpose.take();
    if matches!(state.dialogs.top(), Some(PopupType::FolderScanProgress)) {
        state.dialogs.pop();
    }
    match (outcome, purpose) {
        (Ok(items), Some(ScanPurpose::Compare)) => show_compare(state, &items),
        (Ok(items), Some(ScanPurpose::Sync)) => {
            if let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() {
                dialog.show_review(items);
            }
        }
        (Ok(_), None) | (Err(SyncError::Cancelled), _) => {}
        (Err(err), _) => state.dialogs.push(PopupType::Error(
            t("error_compare_failed").replace("{}", &err.to_string()),
        )),
    }
    state.mark_ui_dirty();
    true
}

/// Tags the differing top-level entries in the left panel and lists them.
fn show_compare(state: &mut AppState, items: &[SyncItem]) {
    let diff = compare_entries(items);
    let left = &mut state.panels.left;
    for entry in diff.iter().filter(|e| e.status != CompareStatus::Equal) {
        if let Some(e) = left.entries.iter().find(|e| e.name == entry.name)
            && left.selected_paths.insert(e.path.clone())
        {
            left.selection_order.push(e.path.clone());
        }
    }
    state.dialogs.replace(PopupType::CompareFoldersResult {
        diff,
        cursor_idx: 0,
    });
}
