use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::Error(_)
            | PopupType::Help { .. }
            | PopupType::Info(_)
            | PopupType::InfoPanel { .. }
            | PopupType::SortModesDialog { .. }
            | PopupType::CompareFoldersResult { .. }
            | PopupType::FileAssociationsDialog { .. }
            | PopupType::ArchiveCommandsMenu { .. }
            | PopupType::QuickViewPanel { .. } => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    state.active_popup = None;
                    return Ok(None);
                }
                Err(())
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
