//! Background loading of the Git panel.
//!
//! Opening the panel, refreshing it after an operation and fetching further
//! log pages never touch the repository on the UI thread: the panel is shown
//! at once (with its previous contents, or empty) and a [`JobSlot`] reads the
//! repository on the blocking pool. Each request bumps the slot generation,
//! so a result from an outdated request is dropped. Results are applied to
//! the Git panel wherever it sits in the dialog stack (an info popup may be
//! on top of it).

#[cfg(test)]
mod tests;

use crate::app::jobs::JobSlot;
use crate::app::state::{AppState, GitPanelState, PopupType};
use crate::config::localization::t;
use crate::git::log::CommitInfo;
use crate::git::snapshot::{self, RepoSnapshot};
use std::path::{Path, PathBuf};

/// Commits read when the panel is refreshed (at least).
pub const LOG_REFRESH_LIMIT: usize = 100;
/// Commits fetched per page when the cursor nears the end of the log.
pub const LOG_PAGE: usize = 50;
/// Rows before the end of the log at which the next page is requested.
const LOG_PREFETCH_ROWS: usize = 10;

/// Result of a snapshot job: requested path, commits asked for, contents.
struct SnapshotResult {
    requested: PathBuf,
    log_limit: usize,
    snapshot: Option<RepoSnapshot>,
}

/// Result of a log-page job.
struct LogPage {
    repo: PathBuf,
    skip: usize,
    commits: Vec<CommitInfo>,
}

/// Background jobs feeding the Git panel.
#[derive(Default)]
pub struct GitPanelLoader {
    snapshot: JobSlot<SnapshotResult>,
    log_page: JobSlot<LogPage>,
    /// Path of the snapshot request in flight.
    loading: Option<PathBuf>,
    /// Local operation (stage, commit, stash, …) in flight.
    pub local: crate::app::git_local::GitLocalJob,
}

impl GitPanelLoader {
    /// `true` while the contents of the panel for `repo_path` are loading.
    pub fn is_loading(&self, repo_path: &Path) -> bool {
        self.snapshot.is_running() && self.loading.as_deref() == Some(repo_path)
    }

    /// `true` while a local operation on `repo_path` is running.
    pub fn is_working(&self, repo_path: &Path) -> bool {
        self.local.is_running_for(repo_path)
    }

    fn start_snapshot(&mut self, path: &Path, log_limit: usize) {
        let requested = path.to_path_buf();
        self.loading = Some(requested.clone());
        self.log_page.cancel();
        self.snapshot.start(move |_| SnapshotResult {
            snapshot: snapshot::load(&requested, log_limit),
            requested,
            log_limit,
        });
    }
}

/// The Git panel in the dialog stack showing `repo_path`, if any.
fn panel_for<'a>(state: &'a mut AppState, repo_path: &Path) -> Option<&'a mut GitPanelState> {
    state.dialogs.iter_mut().find_map(|popup| match popup {
        PopupType::GitPanel(panel) if panel.repo_path == repo_path => Some(panel),
        _ => None,
    })
}

impl AppState {
    /// Opens the Git panel for the repository containing `path`; it fills
    /// in when the background read finishes.
    pub fn open_git_panel_at(&mut self, path: &Path, log_limit: usize) {
        self.dialogs
            .replace(PopupType::GitPanel(GitPanelState::empty(path, 0, 0)));
        self.git_panel.start_snapshot(path, log_limit);
    }

    /// Re-reads the repository in the background. The Git panel on top of
    /// the stack is kept (with its current contents) and switched to
    /// `active_tab`/`cursor_idx`; otherwise an empty panel replaces the
    /// dialogs.
    pub fn refresh_git_panel(&mut self, repo_path: &Path, active_tab: usize, cursor_idx: usize) {
        let loaded = match self.dialogs.top_mut() {
            Some(PopupType::GitPanel(panel)) if panel.repo_path == repo_path => {
                panel.active_tab = active_tab;
                panel.cursor_idx = cursor_idx;
                panel.log_entries.len()
            }
            _ => {
                let panel = GitPanelState::empty(repo_path, active_tab, cursor_idx);
                self.dialogs.replace(PopupType::GitPanel(panel));
                0
            }
        };
        self.git_panel
            .start_snapshot(repo_path, loaded.max(LOG_REFRESH_LIMIT));
    }

    /// Re-reads the Git panel showing `repo_path` wherever it sits in the
    /// dialog stack, keeping its view; does nothing when it was closed.
    pub fn reload_git_panel(&mut self, repo_path: &Path) {
        let Some(panel) = panel_for(self, repo_path) else {
            return;
        };
        let loaded = panel.log_entries.len();
        self.git_panel
            .start_snapshot(repo_path, loaded.max(LOG_REFRESH_LIMIT));
    }

    /// Fetches the next log page when the cursor of the Git panel on top is
    /// close to the end of the loaded history.
    pub fn prefetch_git_log(&mut self) {
        let Some(PopupType::GitPanel(panel)) = self.dialogs.top() else {
            return;
        };
        let near_end = panel.cursor_idx + LOG_PREFETCH_ROWS >= panel.log_entries.len();
        if panel.active_tab != 1
            || panel.log_complete
            || !near_end
            || self.git_panel.log_page.is_running()
            || self.git_panel.snapshot.is_running()
        {
            return;
        }
        let repo = panel.repo_path.clone();
        let skip = panel.log_entries.len();
        self.git_panel.log_page.start(move |_| LogPage {
            commits: snapshot::load_log_page(&repo, skip, LOG_PAGE),
            repo,
            skip,
        });
    }

    /// Applies finished Git panel jobs. Returns `true` when the UI changed.
    pub fn poll_git_panel(&mut self) -> bool {
        let mut changed = false;
        if let Some(result) = self.git_panel.snapshot.poll() {
            self.git_panel.loading = None;
            self.apply_snapshot(result);
            changed = true;
        }
        if let Some(page) = self.git_panel.log_page.poll() {
            if let Some(panel) = panel_for(self, &page.repo)
                && panel.log_entries.len() == page.skip
            {
                panel.log_complete = page.commits.len() < LOG_PAGE;
                panel.log_entries.extend(page.commits);
            }
            changed = true;
        }
        if changed {
            self.mark_ui_dirty();
        }
        changed
    }

    fn apply_snapshot(&mut self, result: SnapshotResult) {
        let Some(snapshot) = result.snapshot else {
            // Not (or no longer) a repository: close the panel with a notice.
            if matches!(self.dialogs.top(), Some(PopupType::GitPanel(p)) if p.repo_path == result.requested)
            {
                self.dialogs.replace(PopupType::Error(t("git_not_a_repo")));
            }
            return;
        };
        if let Some(panel) = panel_for(self, &result.requested) {
            panel.apply_snapshot(snapshot, result.log_limit, t("git_detached_head"));
        }
    }
}
