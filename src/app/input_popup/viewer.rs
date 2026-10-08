use crate::app::context::AppContext;
use crate::app::state::popup::SearchKey;
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::ViewerSearchPrompt(search)) = state.dialogs.top_mut() else {
        return Err(());
    };
    match search.handle_key(&key) {
        SearchKey::Stay => {}
        SearchKey::Close => state.dialogs.clear(),
        SearchKey::Find => {
            let query = search.query.text().to_string();
            let case_sensitive = search.case_sensitive;
            search_viewer(state, query, case_sensitive);
        }
    }
    Ok(None)
}

/// Scrolls the active viewer to the next line containing `query`, wrapping
/// to the top (repeating the same search continues after the current line).
fn search_viewer(state: &mut AppState, query: String, case_sensitive: bool) {
    if query.is_empty() {
        return;
    }
    let Some(Screen::Viewer(vw)) = state.screens.get_mut(state.active_screen_idx) else {
        return;
    };
    let is_repeat = vw.last_search.as_ref() == Some(&query);
    let start_from = if is_repeat { vw.scroll + 1 } else { vw.scroll };
    vw.last_search = Some(query.clone());
    vw.last_case_sensitive = case_sensitive;
    if vw.mode != crate::ui::viewer::ViewerMode::Text {
        return;
    }
    let needle = query.to_lowercase();
    let matches = |l: &str| {
        if case_sensitive {
            l.contains(&query)
        } else {
            l.to_lowercase().contains(&needle)
        }
    };
    let len = vw.lines.len();
    let start = start_from.min(len);
    if let Some(found) = (start..len)
        .chain(0..start)
        .find(|&i| matches(&vw.lines[i]))
    {
        vw.scroll = found;
    }
}
