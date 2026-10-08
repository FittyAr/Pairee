//! Key action handlers for the Status and Log tabs of the Git panel.
//!
//! Repository work runs in the background (`app::git_local`); the handlers
//! only pick the entry under the cursor and open dialogs.

use super::branch_stash::open_checkout;
use crate::app::git_local::{GitContext, reload_panel};
use crate::app::input_popup::git_new_popups::{open_confirm, open_diff, open_name_prompt};
use crate::app::state::popup::{GitCommitPromptState, GitNameAction, GitPromptPopup};
use crate::app::state::{AppState, GitConfirmedAction, PopupType};
use crate::config::localization::t;
use crate::git::log::CommitInfo;
use crate::git::reset::ResetMode;
use crate::git::status::GitFileStatus;
use crossterm::event::KeyCode;
use std::path::Path;

/// Runs a staging-area change in the background, then re-reads the panel.
fn run_and_reload<W>(state: &mut AppState, repo_path: &Path, error_key: &'static str, work: W)
where
    W: FnOnce(&git2::Repository) -> anyhow::Result<()> + Send + 'static,
{
    state.run_git_local(
        repo_path,
        move |repo| work(repo).ctx(error_key),
        reload_panel(repo_path),
    );
}

pub fn handle_status_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    status_entries: &[GitFileStatus],
    cursor_idx: usize,
) -> bool {
    let entry = status_entries.get(cursor_idx);
    match code {
        KeyCode::Char(' ') => {
            if let Some(entry) = entry {
                let path = entry.path.clone();
                if entry.is_staged && !entry.is_unstaged {
                    run_and_reload(state, repo_path, "git_error_unstage_failed", move |r| {
                        crate::git::stage::unstage_file(r, &path)
                    });
                } else {
                    run_and_reload(state, repo_path, "git_error_stage_failed", move |r| {
                        crate::git::stage::stage_file(r, &path)
                    });
                }
            }
        }
        KeyCode::Char('a') => run_and_reload(
            state,
            repo_path,
            "git_error_stage_failed",
            crate::git::stage::stage_all,
        ),
        KeyCode::Char('A') => run_and_reload(
            state,
            repo_path,
            "git_error_unstage_failed",
            crate::git::stage::unstage_all,
        ),
        KeyCode::Char('x') | KeyCode::Delete => {
            if let Some(entry) = entry {
                let msg = t("git_confirm_discard_file").replace("{}", &entry.path);
                let action = GitConfirmedAction::DiscardFile(entry.path.clone());
                open_confirm(state, repo_path, msg, action);
            }
        }
        KeyCode::Char('i' | 'I') => {
            if let Some(entry) = entry {
                let path = entry.path.clone();
                run_and_reload(state, repo_path, "git_error_gitignore_failed", move |r| {
                    crate::git::repo::add_to_gitignore(r, &path)
                });
            }
        }
        KeyCode::Char('X') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path)
                && repo.state() == git2::RepositoryState::Merge
            {
                let msg = t("git_confirm_abort_merge");
                open_confirm(state, repo_path, msg, GitConfirmedAction::AbortMerge);
            }
        }
        KeyCode::Char('c' | 'C') => {
            state.dialogs.open_over(|current_popup| {
                PopupType::GitPrompt(GitPromptPopup::CommitPrompt(GitCommitPromptState {
                    input: Default::default(),
                    repo_path: repo_path.to_path_buf(),
                    is_amend: false,
                    previous_popup: Some(current_popup),
                }))
            });
        }
        KeyCode::Char('d' | 'D') => {
            if let Some(entry) = entry {
                let (path, staged) = (entry.path.clone(), entry.is_staged);
                open_diff(state, repo_path, Some(path.clone()), None, move |r| {
                    crate::git::diff::get_file_diff(r, &path, staged)
                });
            }
        }
        KeyCode::Char('s' | 'S') => open_name_prompt(
            state,
            repo_path,
            GitNameAction::SaveStash {
                include_untracked: false,
            },
        ),
        _ => return false,
    }
    true
}

/// Asks to reset the branch to `commit` with `mode`.
fn confirm_reset(state: &mut AppState, repo_path: &Path, commit: &CommitInfo, mode: ResetMode) {
    let mode_key = match mode {
        ResetMode::Soft => "git_reset_mode_soft",
        ResetMode::Mixed => "git_reset_mode_mixed",
        ResetMode::Hard => "git_reset_mode_hard",
    };
    let msg = t("git_confirm_reset")
        .replace("{commit}", &commit.hash_short)
        .replace("{mode}", &t(mode_key));
    let action = GitConfirmedAction::ResetCommit(commit.hash_full.clone(), mode);
    open_confirm(state, repo_path, msg, action);
}

/// Asks to run `action` on `commit` (message key with `{}` = short hash).
fn confirm_on_commit(
    state: &mut AppState,
    repo_path: &Path,
    commit: &CommitInfo,
    key: &str,
    action: GitConfirmedAction,
) {
    let msg = t(key).replace("{}", &commit.hash_short);
    open_confirm(state, repo_path, msg, action);
}

pub fn handle_log_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    log_entries: &[CommitInfo],
    cursor_idx: usize,
) -> bool {
    let Some(commit) = log_entries.get(cursor_idx) else {
        // Every log action needs a commit; still consume the action keys.
        return matches!(
            code,
            KeyCode::Char('d' | 'D' | 's' | 'x' | 'h' | 'b' | 'B' | 'n' | 'N' | 't' | 'T')
                | KeyCode::Char('c' | 'C' | 'r' | 'R' | 'y' | 'Y')
                | KeyCode::Enter
        );
    };
    let hash = commit.hash_full.clone();
    match code {
        KeyCode::Char('d' | 'D') => {
            let label = Some(commit.hash_short.clone());
            open_diff(state, repo_path, None, label, move |r| {
                crate::git::diff::get_commit_diff(r, &hash)
            });
        }
        KeyCode::Char('s') => confirm_reset(state, repo_path, commit, ResetMode::Soft),
        KeyCode::Char('x') => confirm_reset(state, repo_path, commit, ResetMode::Mixed),
        KeyCode::Char('h') => confirm_reset(state, repo_path, commit, ResetMode::Hard),
        KeyCode::Char('b' | 'B' | 'n' | 'N') => open_name_prompt(
            state,
            repo_path,
            GitNameAction::CreateBranch { start_point: hash },
        ),
        KeyCode::Char('t' | 'T') => {
            open_name_prompt(state, repo_path, GitNameAction::CreateTag { target: hash })
        }
        KeyCode::Char('c' | 'C') => confirm_on_commit(
            state,
            repo_path,
            commit,
            "git_confirm_cherry_pick",
            GitConfirmedAction::CherryPick(hash),
        ),
        KeyCode::Char('r' | 'R') => confirm_on_commit(
            state,
            repo_path,
            commit,
            "git_confirm_revert",
            GitConfirmedAction::Revert(hash),
        ),
        KeyCode::Char('y' | 'Y') => copy_hash(state, commit),
        KeyCode::Enter => open_checkout(state, repo_path, hash, false),
        _ => return false,
    }
    true
}

/// Copies the full hash of `commit` to the clipboard.
fn copy_hash(state: &mut AppState, commit: &CommitInfo) {
    match crate::app::sys_helpers::clipboard::set_text(&commit.hash_full) {
        Ok(()) => {
            let msg = t("git_hash_copied").replace("{}", &commit.hash_short);
            state.dialogs.push(PopupType::Info(msg));
        }
        Err(e) => {
            let msg = t("clipboard_failed").replace("{}", &e.to_string());
            state.dialogs.push(PopupType::Error(msg));
        }
    }
}
