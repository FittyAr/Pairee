//! Shared Git name prompt: create / rename branch, save stash, create tag.

use super::common::restore_previous_and_refresh;
use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::state::popup::{GitNameAction, GitNamePromptState as Prompt, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;

/// Opens the name prompt for `action` over the current dialog (the Git panel).
pub fn open_name_prompt(state: &mut AppState, repo_path: &Path, action: GitNameAction) {
    state.dialogs.open_over(|previous| {
        PopupType::GitPrompt(GitPromptPopup::NamePrompt(Prompt::new(
            action,
            repo_path.to_path_buf(),
            previous,
        )))
    });
}

pub fn handle_prompt(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    // Ctrl+U toggles "include untracked files" in the stash prompt.
    if let GitNameAction::SaveStash { include_untracked } = &mut prompt.action
        && key.code == KeyCode::Char('u')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        *include_untracked = !*include_untracked;
        return Ok(None);
    }
    let field = (prompt.cursor_idx == 0).then_some(&mut prompt.input);
    match Prompt::FORM.handle(&mut prompt.cursor_idx, field, &key) {
        FormKey::Activate(Prompt::BUTTON_CANCEL) | FormKey::Cancel => back(state),
        FormKey::Activate(_) => submit(state),
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

/// Closes the prompt, returning to the dialog it was opened from.
fn back(state: &mut AppState) {
    if let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) = state.dialogs.pop() {
        state.dialogs.replace(*prompt.previous_popup);
    }
}

/// Result of running a prompt's action.
enum Outcome {
    /// Nothing to do (empty or unchanged name): just close.
    Skip,
    /// The repository could not be opened: keep the prompt.
    NoRepo,
    Done,
    Failed(String),
}

fn submit(state: &mut AppState) {
    let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) = state.dialogs.top() else {
        return;
    };
    match run(&prompt.action, &prompt.repo_path, prompt.input.text()) {
        Outcome::Skip => back(state),
        Outcome::NoRepo => {}
        Outcome::Failed(msg) => state.dialogs.replace(PopupType::Error(msg)),
        Outcome::Done => {
            if let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) =
                state.dialogs.pop()
            {
                restore_previous_and_refresh(state, *prompt.previous_popup, &prompt.repo_path);
            }
        }
    }
}

/// Runs `action` with the typed `name` in the repository at `repo_path`.
fn run(action: &GitNameAction, repo_path: &Path, name: &str) -> Outcome {
    let blank = name.trim().is_empty();
    let skip = match action {
        GitNameAction::CreateBranch { .. } | GitNameAction::CreateTag { .. } => blank,
        GitNameAction::RenameBranch { old_name } => blank || name == old_name,
        GitNameAction::SaveStash { .. } => false,
    };
    if skip {
        return Outcome::Skip;
    }
    let Some(mut repo) = crate::git::repo::find_repo(repo_path) else {
        return Outcome::NoRepo;
    };
    let (result, error_key) = match action {
        GitNameAction::CreateBranch { start_point } => (
            crate::git::branches::create_branch(&repo, name, start_point),
            "git_error_create_branch_failed",
        ),
        GitNameAction::RenameBranch { old_name } => (
            crate::git::branches::rename_branch(&repo, old_name, name),
            "git_error_rename_branch_failed",
        ),
        GitNameAction::SaveStash { include_untracked } => {
            let message = (!blank).then_some(name);
            (
                crate::git::stash::stash_save(&mut repo, message, *include_untracked).map(|_| ()),
                "git_error_stash_save_failed",
            )
        }
        GitNameAction::CreateTag { target } => (
            crate::git::tags::create_tag(&repo, name.trim(), target, None).map(|_| ()),
            "git_error_create_tag_failed",
        ),
    };
    match result {
        Ok(()) => Outcome::Done,
        Err(e) => Outcome::Failed(format!("{}: {}", t(error_key), e)),
    }
}
