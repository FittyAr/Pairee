//! One-off changes to a non-local panel source (create a folder or rename
//! an entry on an SFTP server or inside a zip archive, download or upload a
//! file edited with F4, read or change attributes) run as a background job,
//! so network round trips and archive rewrites never block the interface.
//!
//! A job returns a [`VfsFollowUp`] (Command pattern): the step that needs
//! the application state (open the editor, record the journal entry, show
//! a dialog) runs on the UI thread when the job finishes.
//!
//! Jobs run one at a time in the order they were started ([`JobQueue`]):
//! none can be interrupted, so each one's follow-up or error must be
//! delivered, and two rewrites of one archive must not race.

use super::{AppState, PopupType};
use crate::app::jobs::JobQueue;
use crate::config::settings::Settings;
use std::io;

/// UI-thread continuation of a finished background change.
pub type VfsFollowUp = Box<dyn FnOnce(&mut AppState, &Settings) + Send>;

/// The running change and the ones waiting for it.
pub type VfsOps = JobQueue<Result<VfsFollowUp, String>>;

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

    /// Runs `task` in the background after the changes started before it;
    /// its follow-up runs on the UI thread when it finishes (see
    /// [`AppState::poll_vfs_op`]).
    pub fn start_vfs_task(
        &mut self,
        task: impl FnOnce() -> io::Result<VfsFollowUp> + Send + 'static,
    ) {
        self.vfs_op.push(move || task().map_err(|e| e.to_string()));
    }

    /// Applies a finished operation: runs its follow-up or shows its error,
    /// and rereads both panels. Returns `true` when one finished.
    pub fn poll_vfs_op(&mut self, settings: &Settings) -> bool {
        let Some(result) = self.vfs_op.poll() else {
            return false;
        };
        match result {
            Ok(follow_up) => follow_up(self, settings),
            Err(message) => self.dialogs.replace(PopupType::error(message)),
        }
        self.refresh_both_panels(settings.show_hidden);
        self.mark_ui_dirty();
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test(flavor = "multi_thread")]
    async fn every_change_runs_in_order_and_reports() {
        let mut state = AppState::new(".".into(), ".".into());
        let settings = Settings::default();
        let done = Arc::new(Mutex::new(Vec::new()));
        for i in 0..3 {
            let done = done.clone();
            state.start_vfs_task(move || {
                std::thread::sleep(std::time::Duration::from_millis(20));
                done.lock().unwrap().push(i);
                if i == 1 {
                    return Err(io::Error::other("failed"));
                }
                Ok(Box::new(move |state: &mut AppState, _: &Settings| {
                    state.dialogs.push(PopupType::info(format!("done {i}")));
                }) as VfsFollowUp)
            });
        }
        let mut reported = 0;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while reported < 3 {
            assert!(std::time::Instant::now() < deadline, "changes lost");
            if state.poll_vfs_op(&settings) {
                reported += 1;
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }
        assert_eq!(*done.lock().unwrap(), [0, 1, 2]);
        assert!(!state.vfs_op.is_running());
    }
}
