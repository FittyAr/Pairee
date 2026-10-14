//! One-off changes to a non-local panel source (create a folder or rename
//! an entry on an SFTP server or inside a zip archive, download or upload a
//! file edited with F4, read or change attributes) run as a background job,
//! so network round trips and archive rewrites never block the interface.
//!
//! A job returns a [`VfsFollowUp`] (Command pattern): the step that needs
//! the application state (open the editor, record the journal entry, show
//! a dialog) runs on the UI thread when the job finishes.
//!
//! Jobs run one at a time in the order they were started: none can be
//! interrupted, so each one's follow-up or error must be delivered, and
//! two rewrites of one archive must not race.

use super::{AppState, PopupType};
use crate::app::jobs::JobSlot;
use crate::config::settings::Settings;
use std::collections::VecDeque;
use std::io;

/// UI-thread continuation of a finished background change.
pub type VfsFollowUp = Box<dyn FnOnce(&mut AppState, &Settings) + Send>;

type VfsTask = Box<dyn FnOnce() -> io::Result<VfsFollowUp> + Send>;

/// A follow-up that does nothing (the panels are reread anyway).
pub fn no_follow_up() -> VfsFollowUp {
    Box::new(|_, _| {})
}

/// The running change and the ones waiting for it.
#[derive(Default)]
pub struct VfsOps {
    running: JobSlot<Result<VfsFollowUp, String>>,
    queued: VecDeque<VfsTask>,
}

impl std::fmt::Debug for VfsOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VfsOps")
            .field("running", &self.running)
            .field("queued", &self.queued.len())
            .finish()
    }
}

impl VfsOps {
    /// A change is running or waiting.
    pub fn is_running(&self) -> bool {
        self.running.is_running() || !self.queued.is_empty()
    }

    /// Starts the next waiting change once the running one is done.
    fn start_next(&mut self) {
        if self.running.is_running() {
            return;
        }
        if let Some(task) = self.queued.pop_front() {
            self.running
                .start(move |_| task().map_err(|e| e.to_string()));
        }
    }
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
        self.vfs_op.queued.push_back(Box::new(task));
        self.vfs_op.start_next();
    }

    /// Applies a finished operation: runs its follow-up or shows its error,
    /// and rereads both panels. Returns `true` when one finished.
    pub fn poll_vfs_op(&mut self, settings: &Settings) -> bool {
        let result = self.vfs_op.running.poll();
        self.vfs_op.start_next();
        let Some(result) = result else {
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
