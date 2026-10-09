//! The keyboard-shortcuts modal: every command of every context with its
//! keys, searchable by text or by key, where the user rebinds, adds,
//! removes and restores keys, previews other presets and exports their own.

pub mod edit;
pub mod model;
pub mod state;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::embedded::normalize_preset_name;

/// Opens the modal on the active preset.
pub fn open(state: &mut AppState, context: &AppContext) {
    let preset = normalize_preset_name(&context.config.keybindings.preset);
    let rows = edit::rows_for(context, &preset);
    state
        .dialogs
        .replace(PopupType::Shortcuts(Box::new(state::ShortcutsState::new(
            preset, rows,
        ))));
}
