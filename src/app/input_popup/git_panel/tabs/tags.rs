//! Key action handlers for the Tags tab in GitPanel.

use super::branch_stash::open_checkout;
use crate::app::input_popup::git_new_popups::{open_confirm, open_name_prompt};
use crate::app::state::popup::GitNameAction;
use crate::app::state::{AppState, GitConfirmedAction, PopupType};
use crate::config::localization::t;
use crate::git::tags::TagInfo;
use crossterm::event::KeyCode;
use std::path::Path;

pub fn handle_tag_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    tag_entries: &[TagInfo],
    cursor_idx: usize,
) -> bool {
    let tag = tag_entries.get(cursor_idx);
    match code {
        KeyCode::Char('n' | 'N') => open_name_prompt(
            state,
            repo_path,
            GitNameAction::CreateTag {
                target: "HEAD".to_string(),
            },
        ),
        KeyCode::Char('d' | 'D') | KeyCode::Delete => {
            if let Some(tag) = tag {
                let msg = t("git_confirm_delete_tag").replace("{}", &tag.name);
                let action = GitConfirmedAction::DeleteTag(tag.name.clone());
                open_confirm(state, repo_path, msg, action);
            }
        }
        KeyCode::Enter => {
            if let Some(tag) = tag {
                open_checkout(state, repo_path, tag.name.clone(), false);
            }
        }
        KeyCode::Char('u' | 'U') => push_tags(state, repo_path),
        _ => return false,
    }
    true
}

/// Pushes every tag to the default remote (network job with progress).
fn push_tags(state: &mut AppState, repo_path: &Path) {
    let Some(repo) = crate::git::repo::find_repo(repo_path) else {
        return;
    };
    match crate::git::remote::resolve_remote_name(&repo, None) {
        Ok(remote) => state.start_git_op(
            crate::app::git_ops::GitNetOp::PushTags {
                repo_path: repo_path.to_path_buf(),
                remote,
            },
            crate::app::git_ops::FollowUp::Info,
        ),
        Err(_) => state
            .dialogs
            .replace(PopupType::Error(t("git_remote_no_remotes"))),
    }
}
