//! One-off changes to a non-local panel source (create a folder or rename
//! an entry on an SFTP server or inside a zip archive, download or upload a
//! file edited with F4, read or change attributes) run as a background job,
//! so network round trips and archive rewrites never block the interface.
//!
//! A job returns a [`VfsFollowUp`] (Command pattern): the step that needs
//! the application state (open the editor, record the journal entry, show
//! a dialog) runs on the UI thread when the job finishes.

use super::{AppState, PopupType};
use crate::config::settings::Settings;
use std::io;

/// UI-thread continuation of a finished background change.
pub type VfsFollowUp = Box<dyn FnOnce(&mut AppState, &Settings) + Send>;

/// A follow-up that does nothing (the panels are reread anyway).
pub fn no_follow_up() -> VfsFollowUp {
    Box::new(|_, _| {})
}

impl AppState {
    /// Runs `op` in the background; [`AppState::poll_vfs_op`] reports the
    /// outcome and rereads the panels.
    pub fn start_vfs_op(&mut self, op: impl FnOnce() -> io::Result<()> + Send + 'static) {
        self.start_vfs_task(move || op().map(|()| no_follow_up()));
    }

    /// Runs `task` in the background; its follow-up runs on the UI thread
    /// when it finishes (see [`AppState::poll_vfs_op`]).
    pub fn start_vfs_task(
        &mut self,
        task: impl FnOnce() -> io::Result<VfsFollowUp> + Send + 'static,
    ) {
        self.vfs_op
            .start(move |_| task().map_err(|e| e.to_string()));
    }

    /// Applies a finished operation: runs its follow-up or shows its error,
    /// and rereads both panels. Returns `true` when one finished.
    pub fn poll_vfs_op(&mut self, settings: &Settings) -> bool {
        let Some(result) = self.vfs_op.poll() else {
            return false;
        };
        match result {
            Ok(follow_up) => follow_up(self, settings),
            Err(message) => self.dialogs.replace(PopupType::Error(message)),
        }
        self.refresh_both_panels(settings.show_hidden);
        self.mark_ui_dirty();
        true
    }
}
