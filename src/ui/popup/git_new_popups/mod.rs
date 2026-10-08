pub mod confirm;
pub mod diff;
pub mod prompts;
pub mod remote_manage;

use crate::app::state::PopupType;
use crate::app::state::popup::GitPromptPopup;
use crate::config::localization::t;
use crate::config::theme::Theme;
use ratatui::{Frame, layout::Rect};

/// Main entry point to render the new Git popups.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    match popup {
        PopupType::GitPrompt(GitPromptPopup::DiffView(diff_state)) => {
            diff::render_diff_view(f, diff_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::NamePrompt(prompt)) => {
            prompts::render_name_prompt(f, prompt, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::ConfirmAction(action_state)) => {
            confirm::render_confirm_action(f, action_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage_state)) => {
            remote_manage::render_remote_manage(f, manage_state, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add)) => {
            let texts = prompts::PairTexts {
                title: t("git_remote_add_title"),
                labels: [t("git_remote_name_label"), t("git_remote_url_label")],
                hint: t("git_remote_add_hint"),
            };
            prompts::render_field_pair(f, &add.fields, texts, theme, size)
        }
        PopupType::GitPrompt(GitPromptPopup::ClonePrompt(clone)) => {
            let texts = prompts::PairTexts {
                title: t("git_clone_title"),
                labels: [t("git_clone_url_label"), t("git_clone_dir_label")],
                hint: t("git_clone_hint"),
            };
            prompts::render_field_pair(f, &clone.fields, texts, theme, size)
        }
        _ => false,
    }
}
