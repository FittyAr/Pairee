use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles keyboard input for the git commit message prompt.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitCommitPrompt {
        mut input,
        mut cursor_idx,
        repo_path,
    }) = state.active_popup.clone()
    {
        let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Enter => {
                let message = input.trim().to_string();
                if message.is_empty() {
                    state.active_popup = Some(PopupType::Error(
                        crate::config::localization::t("git_commit_empty_msg"),
                    ));
                    return Ok(None);
                }
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    // Stage all
                    if let Err(e) = crate::git::commit::stage_all(&repo) {
                        state.active_popup = Some(PopupType::Error(format!(
                            "{}: {}",
                            crate::config::localization::t("git_error_stage_failed"),
                            e
                        )));
                        return Ok(None);
                    }
                    // Commit
                    match crate::git::commit::commit(
                        &repo,
                        &message,
                        &context.config.settings.git_author_name,
                        &context.config.settings.git_author_email,
                    ) {
                        Ok(oid) => {
                            let short = oid.to_string();
                            let short = &short[..7.min(short.len())];
                            state.active_popup = Some(PopupType::Info(format!(
                                "{} [{}]",
                                crate::config::localization::t("git_commit_success"),
                                short
                            )));
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!(
                                "{}: {}",
                                crate::config::localization::t("git_error_commit_failed"),
                                e
                            )));
                        }
                    }
                } else {
                    state.active_popup = Some(PopupType::Error(
                        crate::config::localization::t("git_not_a_repo"),
                    ));
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
                if cursor_idx > 0 {
                    cursor_idx -= 1;
                }
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

        state.active_popup = Some(PopupType::GitCommitPrompt {
            input,
            cursor_idx,
            repo_path,
        });
        Ok(None)
    } else {
        Err(())
    }
}
