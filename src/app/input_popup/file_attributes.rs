use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Longest octal mode accepted (e.g. `0755`).
const MODE_DIGITS: usize = 4;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::FileAttributesDialog { attrs, mode_input }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match key.code {
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Char(c) if c.is_digit(8) => {
            if mode_input.len() < MODE_DIGITS {
                mode_input.push(c);
            }
        }
        KeyCode::Backspace => {
            mode_input.pop();
        }
        KeyCode::Char('r' | 'R' | ' ') => attrs.readonly = !attrs.readonly,
        KeyCode::Enter => match apply(&attrs.path, mode_input, attrs.readonly) {
            Ok(()) => {
                state.refresh_both_panels(context.config.settings.show_hidden);
                state.dialogs.clear();
            }
            Err(msg) => state.dialogs.replace(PopupType::Error(msg)),
        },
        _ => {}
    }
    Ok(None)
}

/// Sets the octal mode (when typed) and the read-only flag.
fn apply(path: &std::path::Path, mode: &str, readonly: bool) -> Result<(), String> {
    if let Ok(mode) = u32::from_str_radix(mode, 8)
        && let Err(e) = crate::fs::attrs::set_unix_mode(path, mode)
    {
        return Err(t("error_set_unix_mode_failed").replace("{}", &e.to_string()));
    }
    crate::fs::attrs::set_readonly(path, readonly)
        .map_err(|e| t("error_set_readonly_failed").replace("{}", &e.to_string()))
}
