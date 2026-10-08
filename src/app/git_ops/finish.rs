//! Applies a finished Git network operation on the UI thread.

use super::FollowUp;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

/// Drains the Git operation job; returns `true` when it finished this call.
pub fn poll_git_op(state: &mut AppState) -> bool {
    let Some(result) = state.git_op.job.poll() else {
        if state.git_op.is_running() {
            // Progress ticks: keep the gauge moving.
            state.mark_ui_dirty();
        }
        return false;
    };
    if matches!(state.dialogs.top(), Some(PopupType::GitProgress { .. })) {
        state.dialogs.pop();
    }
    let pending = state.git_op.pending.take();
    match (result, pending) {
        (Ok(()), Some((_, follow_up))) => apply_follow_up(state, follow_up),
        (Ok(()), None) => {}
        (Err(Some(err)), pending) => {
            let prefix = pending.map(|(op, _)| t(op.error_key())).unwrap_or_default();
            state
                .dialogs
                .replace(PopupType::Error(format!("{}: {}", prefix, err)));
        }
        (Err(None), _) => state
            .dialogs
            .push(PopupType::Info(t("git_operation_cancelled"))),
    }
    state.mark_ui_dirty();
    true
}

fn apply_follow_up(state: &mut AppState, follow_up: FollowUp) {
    match follow_up {
        FollowUp::Info => {
            state
                .dialogs
                .push(PopupType::Info(t("git_operation_success")));
        }
        FollowUp::RefreshGitPanel {
            repo_path,
            active_tab,
            cursor_idx,
        } => {
            crate::app::input_popup::git_panel::refresh_git_panel(
                state, &repo_path, active_tab, cursor_idx,
            );
            state
                .dialogs
                .push(PopupType::Info(t("git_operation_success")));
        }
        FollowUp::RestorePopup {
            previous,
            repo_path,
        } => {
            crate::app::input_popup::git_new_popups::restore_previous_and_refresh(
                state, *previous, &repo_path,
            );
        }
        FollowUp::Cloned { show_hidden } => {
            state.refresh_both_panels(show_hidden);
            state
                .dialogs
                .replace(PopupType::Info(t("git_clone_success")));
        }
    }
}
