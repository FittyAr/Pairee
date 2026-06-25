pub mod colors;
pub mod confirmations;
pub mod editor_viewer;
pub mod git;
pub mod interface;
pub mod panel;
pub mod plugins;
pub mod system;

pub mod apply;
pub mod editing;
pub mod navigation;
pub mod rows;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ConfigurationDialog {
        active_tab,
        cursor_idx,
        editing_value,
        edit_buffer,
        settings,
        focus_on_tabs,
    }) = state.active_popup.clone()
    {
        if editing_value {
            state.active_popup = editing::handle_editing(
                key,
                active_tab,
                cursor_idx,
                editing_value,
                edit_buffer,
                settings,
                focus_on_tabs,
            );
            return Ok(None);
        }

        return navigation::handle_navigation(
            state,
            key,
            context,
            active_tab,
            cursor_idx,
            editing_value,
            edit_buffer,
            settings,
            focus_on_tabs,
        );
    }
    Err(())
}
