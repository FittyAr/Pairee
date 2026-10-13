pub mod editor;
pub mod terminal;
pub mod viewer;

pub use editor::handle_editor_screen;
pub use terminal::handle_terminal_screen;
pub use viewer::handle_viewer_screen;

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crossterm::event::KeyEvent;

pub fn handle_screen_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    match state.screens.active() {
        Screen::Editor(_) => handle_editor_screen(state, key, context),
        Screen::Viewer(_) => handle_viewer_screen(state, key, context),
        Screen::Terminal(_) => handle_terminal_screen(state, key, context),
        // The panels let the global Action resolver handle it.
        Screen::Panels => Err(()),
    }
}
