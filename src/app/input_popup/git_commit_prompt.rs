use crate::app::context::AppContext;
use crate::app::input_popup::git_new_popups::restore_previous_and_refresh;
use crate::app::state::popup::{GitCommitPromptState, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles keyboard input for the git commit message prompt.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::CommitPrompt(prompt_state))) =
        state.dialogs.top().cloned()
    {
        let mut input = prompt_state.input;
        let mut cursor_idx = prompt_state.cursor_idx;
        let mut is_amend = prompt_state.is_amend;
        let repo_path = prompt_state.repo_path;
        let previous_popup = prompt_state.previous_popup;
        let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                if let Some(prev) = previous_popup {
                    restore_previous_and_refresh(state, *prev, &repo_path);
                } else {
                    state.dialogs.clear();
                }
                return Ok(None);
            }
            KeyCode::Char('a') | KeyCode::Char('A') if is_ctrl => {
                if !is_amend {
                    let has_commits = crate::git::repo::find_repo(&repo_path)
                        .map(|r| r.head().and_then(|h| h.peel_to_commit()).is_ok())
                        .unwrap_or(false);
                    if !has_commits {
                        state
                            .dialogs
                            .push(PopupType::Error(crate::config::localization::t(
                                "git_error_no_commits_to_amend",
                            )));
                        return Ok(None);
                    }
                    is_amend = true;
                    if input.is_empty()
                        && let Some(repo) = crate::git::repo::find_repo(&repo_path)
                        && let Ok(head) = repo.head()
                        && let Ok(parent) = head.peel_to_commit()
                        && let Ok(msg) = parent.message()
                    {
                        input = msg.trim().to_string();
                        cursor_idx = input.len();
                    }
                } else {
                    is_amend = false;
                }
            }
            KeyCode::Enter => {
                let message = input.trim().to_string();
                if message.is_empty() {
                    state
                        .dialogs
                        .replace(PopupType::Error(crate::config::localization::t(
                            "git_commit_empty_msg",
                        )));
                    return Ok(None);
                }
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let statuses = crate::git::status::get_status(&repo);
                    if statuses.is_empty() && !is_amend {
                        state
                            .dialogs
                            .replace(PopupType::Info(crate::config::localization::t(
                                "git_no_changes",
                            )));
                        return Ok(None);
                    }
                    let has_staged = statuses.iter().any(|s| s.is_staged);
                    if !has_staged && !is_amend {
                        // Only auto-stage all if no files were manually staged and it's not an amend
                        if let Err(e) = crate::git::stage::stage_all(&repo) {
                            state.dialogs.replace(PopupType::Error(format!(
                                "{}: {}",
                                crate::config::localization::t("git_error_stage_failed"),
                                e
                            )));
                            return Ok(None);
                        }
                    }
                    // Commit or amend
                    let res = if is_amend {
                        crate::git::commit::commit_amend(
                            &repo,
                            &message,
                            &context.config.settings.git_author_name,
                            &context.config.settings.git_author_email,
                        )
                    } else {
                        crate::git::commit::commit(
                            &repo,
                            &message,
                            &context.config.settings.git_author_name,
                            &context.config.settings.git_author_email,
                        )
                    };
                    match res {
                        Ok(oid) => {
                            let short = oid.to_string();
                            let short = &short[..7.min(short.len())];
                            state.refresh_both_panels(context.config.settings.show_hidden);
                            if let Some(prev) = previous_popup {
                                restore_previous_and_refresh(state, *prev, &repo_path);
                                state.dialogs.push(PopupType::Info(format!(
                                    "{} [{}]",
                                    crate::config::localization::t("git_commit_success"),
                                    short
                                )));
                            } else {
                                state.dialogs.replace(PopupType::Info(format!(
                                    "{} [{}]",
                                    crate::config::localization::t("git_commit_success"),
                                    short
                                )));
                            }
                        }
                        Err(e) => {
                            state.dialogs.replace(PopupType::Error(format!(
                                "{}: {}",
                                crate::config::localization::t("git_error_commit_failed"),
                                e
                            )));
                        }
                    }
                } else {
                    state
                        .dialogs
                        .replace(PopupType::Error(crate::config::localization::t(
                            "git_not_a_repo",
                        )));
                }
                return Ok(None);
            }
            KeyCode::Char(c) if !is_ctrl => {
                input.insert(cursor_idx, c);
                cursor_idx += 1;
            }
            KeyCode::Backspace => {
                if cursor_idx > 0 {
                    cursor_idx -= 1;
                    input.remove(cursor_idx);
                }
            }
            KeyCode::Delete => {
                if cursor_idx < input.len() {
                    input.remove(cursor_idx);
                }
            }
            KeyCode::Left => {
                cursor_idx = cursor_idx.saturating_sub(1);
            }
            KeyCode::Right => {
                if cursor_idx < input.len() {
                    cursor_idx += 1;
                }
            }
            KeyCode::Home => {
                cursor_idx = 0;
            }
            KeyCode::End => {
                cursor_idx = input.len();
            }
            _ => return Ok(None),
        }

        state
            .dialogs
            .replace(PopupType::GitPrompt(GitPromptPopup::CommitPrompt(
                GitCommitPromptState {
                    input,
                    cursor_idx,
                    repo_path,
                    is_amend,
                    previous_popup,
                },
            )));
        Ok(None)
    } else {
        Err(())
    }
}
