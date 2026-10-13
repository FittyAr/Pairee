//! Theme "Color groups" dialog.

use super::color_list::{ColorListKey, back_to_config, handle_key};
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::ColorGroupsDialog;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(ColorGroupsDialog {
        cursor_idx,
        edit,
        theme,
    }) = state.dialogs.top_mut_as::<ColorGroupsDialog>()
    else {
        return Err(());
    };
    match handle_key(theme.as_mut(), cursor_idx, edit, &key) {
        ColorListKey::Handled => {}
        ColorListKey::Back => back_to_config(state, |_| {}),
        ColorListKey::Save => {
            let settings = &mut context.config.settings;
            if settings.theme == "slate" || settings.theme == "classic_blue" {
                settings.theme = "custom".to_string();
            }
            context.config.theme = theme.as_ref().clone();
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
            let theme = context.config.settings.theme.clone();
            back_to_config(state, |draft| draft.theme = theme);
        }
    }
    Ok(None)
}

impl Dialog for ColorGroupsDialog {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
}
