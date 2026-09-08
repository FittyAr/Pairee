pub mod confirm;
pub mod diff;
pub mod prompts;

use crate::app::state::PopupType;
use crate::app::state::popup::GitPromptPopup;
use crate::config::theme::Theme;
use ratatui::{Frame, layout::Rect};

/// Main entry point to render the new Git popups.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    match popup {
        PopupType::GitPrompt(GitPromptPopup::DiffView(diff_state)) => {
            diff::render_diff_view(f, diff_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::BranchCreatePrompt(prompt_state)) => {
            prompts::render_branch_create(f, prompt_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::BranchRenamePrompt(prompt_state)) => {
            prompts::render_branch_rename(f, prompt_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::StashSavePrompt(prompt_state)) => {
            prompts::render_stash_save(f, prompt_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::ConfirmAction(action_state)) => {
            confirm::render_confirm_action(f, action_state, theme, size)
        }
        _ => false,
    }
}
