//! Folder compare and synchronize on the UI side: the background scan
//! ([`JobSlot`] with progress and cancellation), opening the dialog and
//! handing the chosen actions to the Transfer Engine.

mod apply;
mod finish;
#[cfg(test)]
pub(crate) mod tests;

pub use apply::apply_review;
pub use finish::poll_folder_scan;

use crate::app::context::AppContext;
use crate::app::jobs::{JobContext, JobSlot};
use crate::app::state::popup::SyncDialog;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::sync::{ScanObserver, ScanProgress, SyncError, SyncItem, SyncOptions, diff_trees};
use std::path::PathBuf;

/// What the finished scan is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPurpose {
    /// Commands → Compare folders: list and tag the differences.
    Compare,
    /// Synchronize dialog: review and apply actions.
    Sync,
}

impl ScanPurpose {
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Compare => "compare_scan_title",
            Self::Sync => "sync_title",
        }
    }
}

type ScanOutcome = Result<Vec<SyncItem>, SyncError>;

/// The single folder comparison running in the background.
#[derive(Debug, Default)]
pub struct FolderScanState {
    job: JobSlot<ScanOutcome, ScanProgress>,
    purpose: Option<ScanPurpose>,
}

impl FolderScanState {
    pub fn progress(&self) -> Option<ScanProgress> {
        self.job.progress()
    }

    pub fn purpose(&self) -> Option<ScanPurpose> {
        self.purpose
    }

    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }
}

impl ScanObserver for JobContext<ScanProgress> {
    fn is_cancelled(&self) -> bool {
        JobContext::is_cancelled(self)
    }

    fn progress(&self, progress: &ScanProgress) {
        self.report(progress.clone());
    }
}

/// Both panel folders, or `None` (with an error popup) when one of them is
/// remote: comparing reads the local filesystem only.
fn local_roots(state: &mut AppState) -> Option<(PathBuf, PathBuf)> {
    let panels = &state.panels;
    if !panels.left.source.is_local() || !panels.right.source.is_local() {
        state
            .dialogs
            .replace(PopupType::Error(t("compare_local_only")));
        return None;
    }
    Some((
        panels.left.current_path.clone(),
        panels.right.current_path.clone(),
    ))
}

/// Options for a run from the user settings (tolerance, hash algorithm).
pub fn options_from_settings(context: &AppContext) -> SyncOptions {
    SyncOptions {
        compare: crate::fs::compare::CompareOptions::from_settings(&context.config.settings),
        ..SyncOptions::default()
    }
}

/// Commands → Compare folders: compares both panels recursively in the
/// background, then lists the differences.
pub fn start_compare(state: &mut AppState, context: &AppContext) {
    if let Some((left, right)) = local_roots(state) {
        start_scan(
            state,
            ScanPurpose::Compare,
            left,
            right,
            options_from_settings(context),
        );
    }
}

/// Commands → Synchronize folders: opens the options dialog.
pub fn open_dialog(state: &mut AppState, context: &AppContext) {
    if let Some((left, right)) = local_roots(state) {
        let algorithm =
            crate::fs::transfer::transfer_options_from_settings(&context.config.settings)
                .hash_algorithm;
        let dialog = SyncDialog::new(left, right, options_from_settings(context), algorithm);
        state.dialogs.replace(PopupType::SyncDirs(Box::new(dialog)));
    }
}

/// Starts the comparison and shows the progress popup on top.
pub fn start_scan(
    state: &mut AppState,
    purpose: ScanPurpose,
    left: PathBuf,
    right: PathBuf,
    options: SyncOptions,
) {
    state.folder_scan.purpose = Some(purpose);
    state.dialogs.push(PopupType::FolderScanProgress);
    state
        .folder_scan
        .job
        .start(move |ctx| diff_trees(&left, &right, &options, ctx));
    // Inline execution (no runtime) has finished already.
    poll_folder_scan(state);
}

/// Esc in the progress popup: stop the scan and close the popup.
pub fn cancel_scan(state: &mut AppState) {
    state.folder_scan.job.cancel();
    state.folder_scan.purpose = None;
    if matches!(state.dialogs.top(), Some(PopupType::FolderScanProgress)) {
        state.dialogs.pop();
    }
}
