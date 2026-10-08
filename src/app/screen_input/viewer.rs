use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_viewer_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Global pass through
    if key.code == KeyCode::F(12) || (key.code == KeyCode::Tab && is_ctrl) {
        return Err(());
    }

    let Some(vw) = state.active_viewer_mut() else {
        return Err(());
    };
    match key.code {
        KeyCode::Up => vw.scroll_up(1),
        KeyCode::Down => vw.scroll_down(1),
        KeyCode::PageUp => vw.scroll_up(20),
        KeyCode::PageDown => vw.scroll_down(20),
        KeyCode::Home => vw.scroll = 0,
        KeyCode::End => vw.scroll = vw.last_scroll(),
        KeyCode::F(4) => vw.toggle_mode(),
        KeyCode::F(6) => {
            // Viewer → editor: always the built-in editor.
            let path = vw.path.clone();
            crate::app::editor::open::open_in_editor(state, path, &context.config.settings);
        }
        KeyCode::F(7) => state
            .dialogs
            .replace(PopupType::ViewerSearchPrompt(Default::default())),
        KeyCode::F(8) => crate::app::input_popup::viewer::open_encoding_selector(state),
        KeyCode::F(3) => state.repeat_viewer_search(),
        // Esc first stops a running search, then closes the viewer.
        KeyCode::Esc if state.cancel_viewer_search() => {}
        KeyCode::Esc | KeyCode::F(10) => {
            state.cancel_viewer_search();
            state.close_current_screen();
        }
        _ => {}
    }
    Ok(())
}
