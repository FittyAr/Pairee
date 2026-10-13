//! Git dialogs opened over the Git panel, and the clone form.

pub mod confirm;
pub mod diff;
pub mod prompts;
pub mod remote_manage;

use crate::app::state::popup::{
    GitClonePromptState, GitConfirmActionState, GitDiffViewState, GitNamePromptState,
    GitRemoteAddState, GitRemoteManageState,
};
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for GitDiffViewState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        diff::render_diff_view(f, self, cx.theme, cx.area);
    }
}

impl DialogView for GitNamePromptState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        prompts::render_name_prompt(f, self, cx.theme, cx.area);
    }
}

impl DialogView for GitConfirmActionState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        confirm::render_confirm_action(f, self, cx.theme, cx.area);
    }
}

impl DialogView for GitRemoteManageState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        remote_manage::render_remote_manage(f, self, cx.theme, cx.area);
    }
}

impl DialogView for GitRemoteAddState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let texts = prompts::PairTexts {
            title: t("git_remote_add_title"),
            labels: [t("git_remote_name_label"), t("git_remote_url_label")],
            hint: t("git_remote_add_hint"),
        };
        prompts::render_field_pair(f, &self.fields, texts, cx.theme, cx.area);
    }
}

impl DialogView for GitClonePromptState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let texts = prompts::PairTexts {
            title: t("git_clone_title"),
            labels: [t("git_clone_url_label"), t("git_clone_dir_label")],
            hint: t("git_clone_hint"),
        };
        prompts::render_field_pair(f, &self.fields, texts, cx.theme, cx.area);
    }
}
