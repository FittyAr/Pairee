use crate::app::context::AppContext;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top_mut() {
        Some(
            PopupType::Error(_)
            | PopupType::Help { .. }
            | PopupType::Info(_)
            | PopupType::InfoPanel { .. }
            | PopupType::CompareFoldersResult { .. },
        ) if matches!(key.code, KeyCode::Esc | KeyCode::Enter) => {
            // A message over another dialog returns to it.
            if state.dialogs.depth() > 1 {
                state.dialogs.pop();
            } else {
                state.dialogs.clear();
            }
            Ok(None)
        }
        Some(PopupType::QuickViewPanel(qv)) => {
            match key.code {
                KeyCode::Esc => {
                    state.dialogs.clear();
                    state.panels.quick_view_active = false;
                }
                KeyCode::PageUp | KeyCode::PageDown => {
                    let last = match &qv.image_data {
                        Some(img) => (img.height() as usize).div_ceil(2).saturating_sub(5),
                        None => qv.content.len().saturating_sub(QUICK_VIEW_ROWS),
                    };
                    scroll_key(QUICK_VIEW_SCROLL, key.code, &mut qv.scroll, last);
                }
                _ => return Err(()),
            }
            Ok(None)
        }
        _ => Err(()),
    }
}

/// Visible text rows assumed when clamping the quick-view scroll.
const QUICK_VIEW_ROWS: usize = 20;

const QUICK_VIEW_SCROLL: ScrollKeys = ScrollKeys {
    vim: false,
    home_end: false,
    page: 15,
};
