use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::git_local::GitContext;
use crate::app::input_popup::git_new_popups::{close_to, run_and_report};
use crate::app::state::popup::{GitConfirmCheckoutState, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Handles keyboard input for the git checkout confirmation dialog.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if !matches!(
        state.dialogs.top(),
        Some(PopupType::GitPrompt(GitPromptPopup::ConfirmCheckout(_)))
    ) {
        return Err(());
    }
    let Some(confirmed) = confirm_answer(&key, true) else {
        return Ok(None);
    };
    if let Some(PopupType::GitPrompt(GitPromptPopup::ConfirmCheckout(checkout))) =
        state.dialogs.top().cloned()
    {
        if confirmed {
            checkout_target(state, context, checkout);
        } else {
            close_to(state, checkout.previous_popup, &checkout.repo_path);
        }
    }
    Ok(None)
}

/// Checks out the branch / commit in the background over the restored panel.
fn checkout_target(state: &mut AppState, context: &AppContext, checkout: GitConfirmCheckoutState) {
    let GitConfirmCheckoutState {
        target,
        is_branch,
        repo_path,
        previous_popup,
    } = checkout;
    run_and_report(
        state,
        previous_popup,
        &repo_path,
        context.config.settings.show_hidden,
        move |repo| {
            let checked_out = if is_branch {
                crate::git::checkout::checkout_branch(repo, &target)
            } else {
                crate::git::checkout::checkout_commit(repo, &target).map(|()| target.clone())
            }
            .ctx("git_checkout_error")?;
            Ok(PopupType::Info(format!(
                "{}: {}",
                t("git_checkout_success"),
                checked_out
            )))
        },
    );
}
