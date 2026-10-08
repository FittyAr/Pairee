use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::input_popup::git_new_popups::{close_to, finish_with};
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

fn checkout_target(state: &mut AppState, context: &AppContext, checkout: GitConfirmCheckoutState) {
    let Some(repo) = crate::git::repo::find_repo(&checkout.repo_path) else {
        state.dialogs.replace(PopupType::Error(t("git_not_a_repo")));
        return;
    };
    let target = &checkout.target;
    let result = if checkout.is_branch {
        crate::git::checkout::checkout_branch(&repo, target)
    } else {
        crate::git::checkout::checkout_commit(&repo, target).map(|()| target.clone())
    };
    match result {
        Ok(checked_out) => {
            state.refresh_both_panels(context.config.settings.show_hidden);
            let info = PopupType::Info(format!("{}: {}", t("git_checkout_success"), checked_out));
            finish_with(state, checkout.previous_popup, &checkout.repo_path, info);
        }
        Err(e) => state.dialogs.replace(PopupType::Error(format!(
            "{}: {}",
            t("git_checkout_error"),
            e
        ))),
    }
}
