use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::FileAttributesDialog {
        mut attrs,
        mut mode_input,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char(c) if c.is_digit(8) => {
                if mode_input.len() < 4 {
                    mode_input.push(c);
                }
            }
            KeyCode::Backspace => {
                mode_input.pop();
            }
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char(' ') => {
                attrs.readonly = !attrs.readonly;
            }
            KeyCode::Enter => {
                if !mode_input.is_empty() {
                    if let Ok(mode) = u32::from_str_radix(&mode_input, 8) {
                        if let Err(e) = crate::fs::attrs::set_unix_mode(&attrs.path, mode) {
                            state.active_popup = Some(PopupType::Error(
                                t("error_set_unix_mode_failed").replace("{}", &e.to_string()),
                            ));
                            return Ok(None);
                        }
                    }
                }
                if let Err(e) = crate::fs::attrs::set_readonly(&attrs.path, attrs.readonly) {
                    state.active_popup = Some(PopupType::Error(
                        t("error_set_readonly_failed").replace("{}", &e.to_string()),
                    ));
                    return Ok(None);
                }
                state.refresh_both_panels(context.config.settings.show_hidden);
                state.active_popup = None;
                return Ok(None);
            }
            _ => {}
        }
        state.active_popup = Some(PopupType::FileAttributesDialog { attrs, mode_input });
        return Ok(None);
    }
    Err(())
}
