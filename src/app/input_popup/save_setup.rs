use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if !matches!(state.dialogs.top(), Some(PopupType::SaveSetupConfirm)) {
        return Err(());
    }
    match confirm_answer(&key, false) {
        Some(true) => {
            // An explicit "Save setup" is the user's confirmation that an
            // invalid config.toml (already backed up) may be replaced.
            context.config.confirm_settings_overwrite();
            crate::app::sys_helpers::capture_setup(state, &mut context.config.settings);
            let result = match context.config.save() {
                Ok(_) => PopupType::Info(t("setup_saved_success")),
                Err(e) => {
                    PopupType::Error(t("error_save_setup_failed").replace("{}", &e.to_string()))
                }
            };
            state.dialogs.replace(result);
        }
        Some(false) => state.dialogs.clear(),
        None => {}
    }
    Ok(None)
}
