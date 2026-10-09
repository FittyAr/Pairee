//! "Files highlighting" rules dialog.

use super::color_list::{ColorListKey, back_to_config, handle_key};
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Row of the Colors tab that opens this dialog.
const CONFIG_ROW: usize = 2;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::FilesHighlightingDialog {
        cursor_idx,
        edit,
        rules,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match handle_key(rules, cursor_idx, edit, &key) {
        ColorListKey::Handled => {}
        ColorListKey::Back => back_to_config(state, &context.config, CONFIG_ROW),
        ColorListKey::Save => {
            context.config.settings.highlight_rules = rules.clone();
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
            back_to_config(state, &context.config, CONFIG_ROW);
        }
    }
    Ok(None)
}
