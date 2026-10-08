use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SaveSetupConfirm) = state.dialogs.top() {
        match key.code {
            KeyCode::Enter => {
                // An explicit "Save setup" is the user's confirmation that an
                // invalid config.toml (already backed up) may be replaced.
                crate::config::load_guard::confirm_settings_overwrite();
                match context.config.save() {
                    Ok(_) => {
                        state
                            .dialogs
                            .replace(PopupType::Info(t("setup_saved_success")));
                    }
                    Err(e) => {
                        state.dialogs.replace(PopupType::Error(
                            t("error_save_setup_failed").replace("{}", &e.to_string()),
                        ));
                    }
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}
