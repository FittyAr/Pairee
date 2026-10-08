use crate::app::context::AppContext;
use crate::app::list_nav::handle_arrow_nav;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::DriveSelect {
        panel,
        drives,
        cursor_idx,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    if handle_arrow_nav(key.code, cursor_idx, drives.len()) {
        return Ok(None);
    }
    match key.code {
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Enter => {
            let panel = *panel;
            if let Some(drive) = drives.get(*cursor_idx).map(std::path::PathBuf::from) {
                state.panels.side_mut(panel).open_path(drive);
                state.dialogs.clear();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
        KeyCode::Up | KeyCode::Down => {}
        _ => return Err(()),
    }
    Ok(None)
}
