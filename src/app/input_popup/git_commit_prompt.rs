use crate::app::context::AppContext;
use crate::app::git_local::{GitContext, GitFailure};
use crate::app::input_popup::git_new_popups::{close_to, run_and_report};
use crate::app::state::popup::{GitCommitPromptState, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles keyboard input for the git commit message prompt.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPrompt(GitPromptPopup::CommitPrompt(prompt))) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => {
            if let Some(PopupType::GitPrompt(GitPromptPopup::CommitPrompt(prompt))) =
                state.dialogs.pop()
            {
                close_to(state, prompt.previous_popup, &prompt.repo_path);
            }
        }
        KeyCode::Char('a' | 'A') if is_ctrl => {
            if let Err(msg) = toggle_amend(prompt) {
                state.dialogs.push(PopupType::Error(msg));
            }
        }
        KeyCode::Enter => {
            if let Some(PopupType::GitPrompt(GitPromptPopup::CommitPrompt(prompt))) =
                state.dialogs.top().cloned()
            {
                commit(state, context, prompt);
            }
        }
        _ => {
            prompt.input.handle_key(&key);
        }
    }
    Ok(None)
}

/// Ctrl+A: switch amend mode; entering it pre-fills the last commit message.
fn toggle_amend(prompt: &mut GitCommitPromptState) -> Result<(), String> {
    if prompt.is_amend {
        prompt.is_amend = false;
        return Ok(());
    }
    let head_message = crate::git::repo::find_repo(&prompt.repo_path).and_then(|repo| {
        let commit = repo.head().and_then(|h| h.peel_to_commit()).ok()?;
        Some(commit.message().map(|m| m.trim().to_string()).ok())
    });
    let Some(head_message) = head_message else {
        return Err(t("git_error_no_commits_to_amend"));
    };
    prompt.is_amend = true;
    if prompt.input.is_empty()
        && let Some(message) = head_message
    {
        prompt.input.set_text(message);
    }
    Ok(())
}

/// Enter: commits (or amends) in the background over the restored panel.
fn commit(state: &mut AppState, context: &AppContext, prompt: GitCommitPromptState) {
    let message = prompt.input.text().trim().to_string();
    if message.is_empty() {
        state
            .dialogs
            .replace(PopupType::Error(t("git_commit_empty_msg")));
        return;
    }
    let settings = &context.config.settings;
    let author = (
        settings.git_author_name.clone(),
        settings.git_author_email.clone(),
    );
    let is_amend = prompt.is_amend;
    run_and_report(
        state,
        prompt.previous_popup,
        &prompt.repo_path,
        settings.show_hidden,
        move |repo| commit_work(repo, &message, is_amend, &author),
    );
}

/// Stages everything when nothing is staged (not when amending), then
/// commits. Returns the message to show.
fn commit_work(
    repo: &mut git2::Repository,
    message: &str,
    is_amend: bool,
    (name, email): &(String, String),
) -> Result<PopupType, GitFailure> {
    let statuses = crate::git::status::get_status(repo);
    if statuses.is_empty() && !is_amend {
        return Ok(PopupType::Info(t("git_no_changes")));
    }
    if !is_amend && !statuses.iter().any(|s| s.is_staged) {
        crate::git::stage::stage_all(repo).ctx("git_error_stage_failed")?;
    }
    let commit_fn = if is_amend {
        crate::git::commit::commit_amend
    } else {
        crate::git::commit::commit
    };
    let oid = commit_fn(repo, message, name, email)
        .ctx("git_error_commit_failed")?
        .to_string();
    Ok(PopupType::Info(format!(
        "{} [{}]",
        t("git_commit_success"),
        &oid[..7.min(oid.len())]
    )))
}
