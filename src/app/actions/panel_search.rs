//! In-panel search: `find_in_panel` (Vim / yazi `/`), `find_next` /
//! `find_prev` (`n` / `N`) and Norton Commander's Alt+letter quick search.

use crate::app::input::panel_find::{Direction, NameMatch, find};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::keybindings::Action;

/// The last confirmed search, repeated by `find_next` / `find_prev`.
#[derive(Debug, Clone)]
pub struct PanelQuery {
    pub text: String,
    pub how: NameMatch,
}

pub fn handle(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::FindInPanel => open(state, NameMatch::Substring, None),
        Action::FindNext => repeat(state, Direction::Forward),
        Action::FindPrev => repeat(state, Direction::Backward),
        _ => return false,
    }
    true
}

/// Opens the search prompt, optionally with its first character typed
/// (the quick search starts with the letter pressed with Alt).
pub fn open(state: &mut AppState, how: NameMatch, first: Option<char>) {
    let origin = state.get_active_panel().cursor_index;
    let mut query = TextField::default();
    if let Some(c) = first {
        query.insert_char(c);
        jump(state, &c.to_string(), how, origin, Direction::Forward);
    }
    state
        .dialogs
        .replace(PopupType::PanelSearch { query, how, origin });
}

/// Moves the cursor to the next match of `text` from `start`; `false` when
/// nothing matches.
pub fn jump(
    state: &mut AppState,
    text: &str,
    how: NameMatch,
    start: usize,
    direction: Direction,
) -> bool {
    let panel = state.get_active_panel_mut();
    let names: Vec<&str> = panel.entries.iter().map(|e| e.name.as_str()).collect();
    match find(&names, start, text, how, direction) {
        Some(i) => {
            panel.cursor_index = i;
            true
        }
        None => false,
    }
}

/// The index after (or before) the cursor, for stepping between matches.
pub fn step_from(state: &AppState, direction: Direction) -> usize {
    let panel = state.get_active_panel();
    let len = panel.entries.len().max(1);
    match direction {
        Direction::Forward => panel.cursor_index + 1,
        Direction::Backward => panel.cursor_index + len - 1,
    }
}

fn repeat(state: &mut AppState, direction: Direction) {
    if let Some(query) = state.last_panel_search.clone() {
        let start = step_from(state, direction);
        jump(state, &query.text, query.how, start, direction);
    }
}
