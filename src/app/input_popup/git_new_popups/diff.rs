//! Handles key input for the GitDiffView popup.

use crate::app::context::AppContext;
use crate::app::state::popup::{GitDiffViewState, GitPromptPopup};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_diff(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::DiffView(diff_state))) =
        state.dialogs.top().cloned()
    {
        let repo_path = diff_state.repo_path;
        let file_path = diff_state.file_path;
        let commit_hash = diff_state.commit_hash;
        let diff_content = diff_state.diff_content;
        let mut scroll_y = diff_state.scroll_y;
        let previous_popup = diff_state.previous_popup;
        let lines_count = diff_content.lines().count();

        match key.code {
            KeyCode::Up => {
                scroll_y = scroll_y.saturating_sub(1);
            }
            KeyCode::Down => {
                if scroll_y + 5 < lines_count {
                    scroll_y += 1;
                }
            }
            KeyCode::PageUp => {
                scroll_y = scroll_y.saturating_sub(15);
            }
            KeyCode::PageDown => {
                scroll_y = (scroll_y + 15).min(lines_count.saturating_sub(5));
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.dialogs.replace(*previous_popup);
                return Ok(None);
            }
            _ => {}
        }

        state
            .dialogs
            .replace(PopupType::GitPrompt(GitPromptPopup::DiffView(
                GitDiffViewState {
                    repo_path,
                    file_path,
                    commit_hash,
                    diff_content,
                    scroll_y,
                    previous_popup,
                },
            )));
        Ok(None)
    } else {
        Err(())
    }
}
