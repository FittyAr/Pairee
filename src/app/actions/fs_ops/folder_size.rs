//! Folder size calculation and the disk usage view.

use crate::app::state::{AppState, PopupType};

/// "Calculate folder sizes": the selected folders, or every listed folder.
pub fn calculate(state: &mut AppState) -> bool {
    state.get_active_panel_mut().calculate_targeted_dir_sizes();
    true
}

/// Opens the disk usage view on the active panel's folder.
pub fn open_disk_usage(state: &mut AppState) -> bool {
    let panel = state.get_active_panel();
    let root = panel.current_path.clone();
    let source = panel.source.clone();
    state.disk_usage.open(root, source);
    state.dialogs.replace(PopupType::DiskUsage);
    true
}
