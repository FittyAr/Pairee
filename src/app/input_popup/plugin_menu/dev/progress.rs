//! Progress reporting for the Developer Tools module: `begin_dev_op`
//! wires the progress channel on `AppState`, `progress_status` emits
//! a coarse status update, and `dev_op_running` reports whether a dev
//! operation is currently in flight.

use crate::app::state::{AppState, DevProgress, PopupType};

/// Start a Developer Tools async operation: wire up the progress channel on
/// `state`, flip the popup into the "loading" state, and return the sender
/// half so the caller can spawn the work.
pub fn begin_dev_op(
    state: &mut AppState,
    initial_status: String,
) -> tokio::sync::mpsc::UnboundedSender<DevProgress> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<DevProgress>();
    state.dev_progress_rx = Some(rx);
    if let Some(PopupType::PluginMenu {
        dev_loading,
        dev_loading_status,
        dev_loading_progress,
        ..
    }) = &mut state.active_popup
    {
        *dev_loading = true;
        *dev_loading_status = initial_status;
        *dev_loading_progress = None;
    }
    tx
}

/// Emit a coarse status update over the given progress sender (if any).
pub fn progress_status(
    tx: &Option<tokio::sync::mpsc::UnboundedSender<DevProgress>>,
    status: String,
) {
    if let Some(tx) = tx {
        let _ = tx.send(DevProgress {
            status,
            current: None,
            total: None,
            done: false,
            result: None,
            error: None,
        });
    }
}

/// Returns true if a Developer Tools operation is currently in flight.
pub fn dev_op_running(state: &AppState) -> bool {
    if let Some(PopupType::PluginMenu { dev_loading, .. }) = &state.active_popup {
        *dev_loading
    } else {
        false
    }
}
