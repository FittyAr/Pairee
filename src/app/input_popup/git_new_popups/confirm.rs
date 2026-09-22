//! Handles key input for the generic GitConfirmAction dialog.

use super::common::restore_previous_and_refresh;
use crate::app::context::AppContext;
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_confirm_action(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(action_state))) =
        state.dialogs.top().cloned()
    {
        let repo_path = action_state.repo_path;
        let action = action_state.action;
        let previous_popup = action_state.previous_popup;
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                match action {
                    GitConfirmedAction::DeleteBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::branches::delete_branch(&repo, &name) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_delete_branch_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::MergeBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::merge::merge(&repo, &name) {
                                Ok(_analysis) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    let has_conflicts = repo
                                        .index()
                                        .map(|idx| idx.has_conflicts())
                                        .unwrap_or(false);
                                    if has_conflicts {
                                        state.dialogs.replace(PopupType::Error(t(
                                            "git_error_merge_conflicts",
                                        )));
                                    } else {
                                        state
                                            .dialogs
                                            .replace(PopupType::Info(t("git_merge_success")));
                                    }
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_merge_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::StashDrop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_drop(&mut repo, index) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_stash_drop_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::StashPop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_pop(&mut repo, index) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_stash_pop_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::ResetCommit(hash, mode) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::reset::reset(&repo, &hash, mode) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_reset_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::DiscardFile(path) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stage::discard_file_changes(&repo, &path) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_discard_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::DeleteRemoteBranch { remote, branch } => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::remote::delete_remote_branch(&repo, &remote, &branch)
                            {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_delete_remote_branch_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::DeleteRemote(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::remote::delete_remote(&repo, &name) {
                                Ok(_) => {
                                    if let PopupType::GitPrompt(GitPromptPopup::RemoteManage(
                                        mut manage_state,
                                    )) = *previous_popup
                                    {
                                        manage_state.remotes =
                                            crate::git::remote::list_remotes(&repo)
                                                .unwrap_or_default();
                                        if manage_state.selected_idx >= manage_state.remotes.len()
                                            && !manage_state.remotes.is_empty()
                                        {
                                            manage_state.selected_idx =
                                                manage_state.remotes.len() - 1;
                                        }
                                        state.dialogs.replace(PopupType::GitPrompt(
                                            GitPromptPopup::RemoteManage(manage_state),
                                        ));
                                    } else {
                                        restore_previous_and_refresh(
                                            state,
                                            *previous_popup,
                                            &repo_path,
                                        );
                                    }
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_delete_remote_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::AbortMerge => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::merge::abort_merge(&repo) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_abort_merge_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::CherryPick(hash) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::cherry_pick::cherry_pick(&repo, &hash) {
                                Ok(_) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    let has_conflicts = repo
                                        .index()
                                        .map(|idx| idx.has_conflicts())
                                        .unwrap_or(false);
                                    if has_conflicts {
                                        state.dialogs.replace(PopupType::Error(t(
                                            "git_error_cherry_pick_conflicts",
                                        )));
                                    } else {
                                        state.dialogs.replace(PopupType::Info(
                                            crate::config::localization::t("git_operation_success"),
                                        ));
                                    }
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_cherry_pick_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::Revert(hash) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::revert::revert(&repo, &hash) {
                                Ok(_) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    let has_conflicts = repo
                                        .index()
                                        .map(|idx| idx.has_conflicts())
                                        .unwrap_or(false);
                                    if has_conflicts {
                                        state.dialogs.replace(PopupType::Error(t(
                                            "git_error_revert_conflicts",
                                        )));
                                    } else {
                                        state.dialogs.replace(PopupType::Info(
                                            crate::config::localization::t("git_operation_success"),
                                        ));
                                    }
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_revert_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::RebaseBranch(onto_name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::rebase::rebase_branch(&repo, &onto_name) {
                                Ok(_) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    state.dialogs.replace(PopupType::Info(
                                        crate::config::localization::t("git_operation_success"),
                                    ));
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_rebase_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::StashClear => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_clear(&mut repo) {
                                Ok(_) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    state.dialogs.replace(PopupType::Info(
                                        crate::config::localization::t("git_operation_success"),
                                    ));
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_stash_clear_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::DeleteTag(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::tags::delete_tag(&repo, &name) {
                                Ok(_) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    state.dialogs.replace(PopupType::Info(
                                        crate::config::localization::t("git_operation_success"),
                                    ));
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "{}: {}",
                                    t("git_error_delete_tag_failed"),
                                    e
                                ))),
                            }
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.dialogs.replace(*previous_popup);
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}
