//! Handles key input for the GitDiffView popup.

use crate::app::context::AppContext;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const DIFF_SCROLL: ScrollKeys = ScrollKeys {
    vim: false,
    home_end: true,
    page: 15,
};

pub fn handle_diff(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPrompt(GitPromptPopup::DiffView(diff))) = state.dialogs.top_mut() else {
        return Err(());
    };
    let last = diff.diff_content.lines().count().saturating_sub(1);
    if !scroll_key(DIFF_SCROLL, key.code, &mut diff.scroll_y, last)
        && matches!(key.code, KeyCode::Esc | KeyCode::Char('q' | 'Q'))
        && let Some(PopupType::GitPrompt(GitPromptPopup::DiffView(diff))) = state.dialogs.pop()
    {
        state.dialogs.replace(*diff.previous_popup);
    }
    Ok(None)
}
