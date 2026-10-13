//! Handles key input for the GitDiffView popup.

use crate::app::context::AppContext;
use crate::app::git_local::GitContext;
use crate::app::input_popup::diff_view::{DiffKey, handle_key};
use crate::app::state::popup::{Dialog, DiffViewer, GitDiffViewState, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::diff::DiffDocument;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;
use std::path::Path;

pub fn handle_diff(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitDiffView(diff)) = state.dialogs.top_mut() else {
        return Err(());
    };
    if handle_key(&mut diff.viewer, key.code) == DiffKey::Close {
        state.dialogs.pop();
    }
    Ok(None)
}

/// Computes a diff in the background (`work`) and opens the diff view over
/// the current dialog (the Git panel) when it is ready. `file_path` /
/// `commit_hash` label the view.
pub fn open_diff<W>(
    state: &mut AppState,
    repo_path: &Path,
    file_path: Option<String>,
    commit_hash: Option<String>,
    work: W,
) where
    W: FnOnce(&git2::Repository) -> anyhow::Result<String> + Send + 'static,
{
    let repo_path_buf = repo_path.to_path_buf();
    state.run_git_local(
        repo_path,
        move |repo| work(repo).ctx("git_error_diff_failed"),
        move |state, diff_content| {
            state.dialogs.push(GitDiffViewState {
                repo_path: repo_path_buf,
                file_path,
                commit_hash,
                viewer: DiffViewer::new(DiffDocument::from_unified(&diff_content)),
            });
        },
    );
}

impl Dialog for GitDiffViewState {
    fn key_handler(&self) -> KeyHandler {
        handle_diff
    }
}
