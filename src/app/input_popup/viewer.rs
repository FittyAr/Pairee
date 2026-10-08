use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::SearchKey;
use crate::app::state::{AppState, PopupType};
use crate::fs::text::ENCODINGS;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Keys of the viewer's find dialog and encoding selector.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top_mut() {
        Some(PopupType::ViewerSearchPrompt(search)) => match search.handle_key(&key) {
            SearchKey::Stay => {}
            SearchKey::Close => state.dialogs.clear(),
            SearchKey::Find => {
                let query = search.query.text().to_string();
                let case_sensitive = search.case_sensitive;
                state.search_viewer(query, case_sensitive);
            }
        },
        Some(PopupType::ViewerEncoding { cursor_idx }) => {
            match list_key(ListKeys::FULL, key.code, cursor_idx, ENCODINGS.len()) {
                ListKey::Moved | ListKey::Other => {}
                ListKey::Close => state.dialogs.clear(),
                ListKey::Activate(idx) => {
                    state.dialogs.clear();
                    if let (Some(vw), Some(encoding)) =
                        (state.active_viewer_mut(), ENCODINGS.get(idx))
                    {
                        vw.set_encoding(encoding);
                    }
                }
            }
        }
        _ => return Err(()),
    }
    Ok(None)
}

/// Opens the encoding selector on the active viewer's current encoding.
pub fn open_encoding_selector(state: &mut AppState) {
    let Some(vw) = state.active_viewer_mut() else {
        return;
    };
    let current = vw.doc.encoding();
    let cursor_idx = ENCODINGS.iter().position(|e| *e == current).unwrap_or(0);
    state
        .dialogs
        .replace(PopupType::ViewerEncoding { cursor_idx });
}
