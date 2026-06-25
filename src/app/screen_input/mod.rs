pub mod editor;
pub mod viewer;

pub use editor::handle_editor_screen;
pub use viewer::handle_viewer_screen;

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crossterm::event::KeyEvent;

pub fn handle_screen_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    // Determine the type of the active screen
    let is_editor = matches!(
        state.screens.get(state.active_screen_idx),
        Some(Screen::Editor(_))
    );
    let is_viewer = matches!(
        state.screens.get(state.active_screen_idx),
        Some(Screen::Viewer(_))
    );

    if is_editor {
        return handle_editor_screen(state, key, context);
    } else if is_viewer {
        return handle_viewer_screen(state, key, context);
    }

    // Panels and Terminal let the global Action resolver handle it
    Err(())
}
