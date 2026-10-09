//! Viewer screen keys: global panel actions pass through, everything else
//! runs the command the `[viewer]` keymap section binds.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::screens::ViewerAction;
use crossterm::event::KeyEvent;

/// Lines moved by PgUp / PgDn.
const PAGE: usize = 20;

pub fn handle_viewer_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    if context.resolver.global_action(key).is_some() || state.active_viewer_mut().is_none() {
        return Err(());
    }
    if let Some(command) = context.resolver.viewer.dispatch(key) {
        run(state, command, context);
    }
    Ok(())
}

fn run(state: &mut AppState, command: ViewerAction, context: &AppContext) {
    use ViewerAction as V;
    let Some(vw) = state.active_viewer_mut() else {
        return;
    };
    match command {
        V::LineUp => vw.scroll_up(1),
        V::LineDown => vw.scroll_down(1),
        V::PageUp => vw.scroll_up(PAGE),
        V::PageDown => vw.scroll_down(PAGE),
        V::Top => vw.scroll = 0,
        V::Bottom => vw.scroll = vw.last_scroll(),
        V::ToggleHex => vw.toggle_mode(),
        V::Edit => {
            // Viewer → editor: always the built-in editor.
            let path = vw.path.clone();
            crate::app::actions::fs_ops::edit::edit_file(state, path, &context.config.settings);
        }
        V::Search => state
            .dialogs
            .replace(PopupType::ViewerSearchPrompt(Default::default())),
        V::Encoding => crate::app::input_popup::viewer::open_encoding_selector(state),
        V::SearchNext => state.repeat_viewer_search(),
        // A running search stops first; the next Quit closes the viewer.
        V::Quit if state.cancel_viewer_search() => {}
        V::Quit => state.close_current_screen(),
    }
}
