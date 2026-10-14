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
        } else if state.git_op.job.take_panicked() {
            // Ended without a result: never leave the progress popup up.
            state.git_op.pending = None;
            if matches!(state.dialogs.top(), Some(PopupType::GitProgress(_))) {
                state.dialogs.pop();
            }
            state
                .dialogs
                .push(PopupType::error(t("background_job_failed")));
            state.mark_ui_dirty();
            return true;
        }
        return false;
    };
    if matches!(state.dialogs.top(), Some(PopupType::GitProgress(_))) {
        state.dialogs.pop();
    }
    let pending = state.git_op.pending.take();
    match (result, pending) {
        (Ok(conflicts), Some((_, follow_up))) => {
            apply_follow_up(state, follow_up, conflicts.is_empty());
            if !conflicts.is_empty() {
                let summary = super::conflict_summary(&conflicts);
                let msg = t("git_pull_conflicts").replace("{}", &summary);
                state.dialogs.push(PopupType::error(msg));
            }
        }
        (Ok(_), None) => {}
        (Err(Some(err)), pending) => {
            let prefix = pending.map(|(op, _)| t(op.error_key())).unwrap_or_default();
            state
                .dialogs
                .replace(PopupType::error(format!("{}: {}", prefix, err)));
        }
        (Err(None), _) => state
            .dialogs
            .push(PopupType::info(t("git_operation_cancelled"))),
    }
    state.mark_ui_dirty();
    true
}

/// Applies `follow_up`; the success message only when the operation ended
/// `clean` (a conflicted pull shows its own message instead).
fn apply_follow_up(state: &mut AppState, follow_up: FollowUp, clean: bool) {
    match follow_up {
        FollowUp::Info => {
            state
                .dialogs
                .push(PopupType::info(t("git_operation_success")));
        }
        FollowUp::RefreshGitPanel {
            repo_path,
            active_tab,
            cursor_idx,
        } => {
            state.refresh_git_panel(&repo_path, active_tab, cursor_idx);
            if clean {
                state
                    .dialogs
                    .push(PopupType::info(t("git_operation_success")));
            }
        }
        FollowUp::RefreshPanelOnTop { repo_path } => {
            crate::app::input_popup::git_new_popups::refresh_panel_on_top(state, &repo_path);
        }
        FollowUp::Cloned { show_hidden } => {
            state.refresh_both_panels(show_hidden);
            state
                .dialogs
                .replace(PopupType::info(t("git_clone_success")));
        }
    }
}
