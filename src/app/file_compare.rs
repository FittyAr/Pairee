//! Compare files (outside Git): picks the two files, compares them in the
//! background ([`crate::diff::files::compare_files`]) and fills the
//! Compare files view when the result arrives.

use crate::app::jobs::JobSlot;
use crate::app::state::popup::{CompareStatus, DiffViewer, FileCompareView};
use crate::app::state::{AppState, PanelState, PopupType};
use crate::config::localization::t;
use crate::diff::files::{FileComparison, FileSide, compare_files};
use crate::fs::vfs::PanelSource;
use std::path::PathBuf;

/// The running comparison.
#[derive(Debug, Default)]
pub struct FileCompareState {
    job: JobSlot<Result<FileComparison, String>>,
}

impl FileCompareState {
    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }

    pub fn cancel(&mut self) {
        self.job.cancel();
    }
}

/// A file to compare and where it lives.
type Target = (PanelSource, PathBuf);

/// Opens Compare files on the two selected files of the active panel, or
/// on the file under the cursor of each panel.
pub fn open(state: &mut AppState) -> bool {
    let Some((old, new)) = targets(state) else {
        state
            .dialogs
            .replace(PopupType::error(t("compare_files_need_two")));
        return true;
    };
    state.dialogs.replace(FileCompareView {
        old: old.1.clone(),
        new: new.1.clone(),
        status: CompareStatus::Running,
        viewer: DiffViewer::default(),
    });
    state.file_compare.job.start(move |ctx| {
        let (old_vfs, new_vfs) = (old.0.vfs(), new.0.vfs());
        compare_files(
            FileSide {
                vfs: old_vfs.as_ref(),
                path: &old.1,
            },
            FileSide {
                vfs: new_vfs.as_ref(),
                path: &new.1,
            },
            &|| ctx.is_cancelled(),
        )
        .map_err(|e| e.to_string())
    });
    poll(state);
    true
}

/// The two files: two selected files of the active panel, else the files
/// under the cursor of the active (old) and the other (new) panel.
fn targets(state: &AppState) -> Option<(Target, Target)> {
    let active = state.get_active_panel();
    let selected: Vec<PathBuf> = active
        .selection_order
        .iter()
        .filter(|path| is_file(active, path))
        .cloned()
        .collect();
    if let [old, new] = selected.as_slice() {
        let source = active.source.clone();
        return Some(((source.clone(), old.clone()), (source, new.clone())));
    }
    Some((
        cursor_file(active)?,
        cursor_file(state.get_passive_panel())?,
    ))
}

fn is_file(panel: &PanelState, path: &std::path::Path) -> bool {
    panel
        .entries
        .iter()
        .any(|e| e.path == path && !e.is_dir && e.name != "..")
}

fn cursor_file(panel: &PanelState) -> Option<Target> {
    let entry = panel.entries.get(panel.cursor_index)?;
    (!entry.is_dir).then(|| (panel.source.clone(), entry.path.clone()))
}

/// Installs a finished comparison in the open view. Returns `true` when
/// one arrived.
pub fn poll(state: &mut AppState) -> bool {
    let Some(result) = state.file_compare.job.poll() else {
        return false;
    };
    let view = state.dialogs.iter_mut().find_map(|dialog| match dialog {
        PopupType::FileCompareView(view) => Some(view),
        _ => None,
    });
    if let Some(view) = view {
        view.status = match result {
            Ok(FileComparison::Text(doc)) => {
                view.viewer = DiffViewer::new(doc);
                CompareStatus::Text
            }
            Ok(FileComparison::Binary(binary)) => CompareStatus::Binary(binary),
            Err(error) => CompareStatus::Failed(error),
        };
    }
    true
}
