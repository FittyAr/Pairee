//! Shared Git name prompt: create / rename branch, save stash, create tag.

use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::git_local::{GitContext, GitFailure, reload_panel};
use crate::app::state::popup::{GitNameAction, GitNamePromptState as Prompt, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git2::Repository;
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

/// OK: runs the action in the background over the restored Git panel.
fn submit(state: &mut AppState) {
    let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) = state.dialogs.top() else {
        return;
    };
    if is_noop(&prompt.action, prompt.input.text()) {
        back(state);
        return;
    }
    let Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt))) = state.dialogs.pop() else {
        return;
    };
    let Prompt {
        action,
        input,
        repo_path,
        previous_popup,
        ..
    } = prompt;
    state.dialogs.replace(*previous_popup);
    let name = input.text().to_string();
    state.run_git_local(
        &repo_path,
        move |repo| run(&action, repo, &name),
        reload_panel(&repo_path),
    );
}

/// `true` when the typed `name` leaves nothing to do (empty or unchanged).
fn is_noop(action: &GitNameAction, name: &str) -> bool {
    let blank = name.trim().is_empty();
    match action {
        GitNameAction::CreateBranch { .. } | GitNameAction::CreateTag { .. } => blank,
        GitNameAction::RenameBranch { old_name } => blank || name == old_name,
        GitNameAction::SaveStash { .. } => false,
    }
}

/// Runs `action` with the typed `name` (on the job thread).
fn run(action: &GitNameAction, repo: &mut Repository, name: &str) -> Result<(), GitFailure> {
    use crate::git::{branches, stash, tags};
    match action {
        GitNameAction::CreateBranch { start_point } => {
            branches::create_branch(repo, name, start_point).ctx("git_error_create_branch_failed")
        }
        GitNameAction::RenameBranch { old_name } => {
            branches::rename_branch(repo, old_name, name).ctx("git_error_rename_branch_failed")
        }
        GitNameAction::SaveStash { include_untracked } => {
            let message = (!name.trim().is_empty()).then_some(name);
            stash::stash_save(repo, message, *include_untracked)
                .map(|_| ())
                .ctx("git_error_stash_save_failed")
        }
        GitNameAction::CreateTag { target } => tags::create_tag(repo, name.trim(), target, None)
            .map(|_| ())
            .ctx("git_error_create_tag_failed"),
    }
}
