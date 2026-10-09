//! Keys of the in-panel search prompt: typing jumps to the first match from
//! where the search started, Up / Down step between matches, Enter keeps
//! the position (and the query for `n` / `N`), Esc gives up. A Vim-style
//! search returns to where it started; a quick search stays.

use crate::app::actions::panel_search::{PanelQuery, jump, step_from};
use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::input::panel_find::{Direction, NameMatch};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::PanelSearch { query, how, origin }) = state.dialogs.top_mut() else {
        return Err(());
    };
    let (how, origin) = (*how, *origin);
    let step = match key.code {
        KeyCode::Down => Some(Direction::Forward),
        KeyCode::Up => Some(Direction::Backward),
        _ => None,
    };
    if let Some(direction) = step {
        let text = query.text().to_string();
        let start = step_from(state, direction);
        jump(state, &text, how, start, direction);
        return Ok(None);
    }
    match field_key(query, &key) {
        FieldKey::Submit => {
            let text = query.text().to_string();
            state.dialogs.pop();
            if !text.is_empty() {
                state.last_panel_search = Some(PanelQuery { text, how });
            }
        }
        FieldKey::Cancel => {
            state.dialogs.pop();
            if how == NameMatch::Substring {
                state.get_active_panel_mut().cursor_index = origin;
            }
        }
        FieldKey::Handled => {
            let text = query.text().to_string();
            jump(state, &text, how, origin, Direction::Forward);
        }
        FieldKey::Other => {}
    }
    Ok(None)
}
