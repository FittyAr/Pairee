//! One-off changes to a non-local panel source (create a folder on an SFTP
//! server or inside a zip archive) run as a background job, so network
//! round trips and archive rewrites never block the interface.

use super::{AppState, PopupType};
use std::io;

impl AppState {
    /// Runs `op` in the background; [`AppState::poll_vfs_op`] reports the
    /// outcome and rereads the panels.
    pub fn start_vfs_op(&mut self, op: impl FnOnce() -> io::Result<()> + Send + 'static) {
        self.vfs_op.start(move |_| op().map_err(|e| e.to_string()));
    }

    /// Applies a finished operation: shows its error, if any, and rereads
    /// both panels. Returns `true` when one finished.
    pub fn poll_vfs_op(&mut self, show_hidden: bool) -> bool {
        let Some(result) = self.vfs_op.poll() else {
            return false;
        };
        if let Err(message) = result {
            self.dialogs.replace(PopupType::Error(message));
        }
        self.refresh_both_panels(show_hidden);
        self.mark_ui_dirty();
        true
    }
}
