use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.active_popup {
        Some(PopupType::YaziSortPopup) => {
            state.active_popup = None; // always close on keypress
            if let KeyCode::Char(c) = key.code {
                let action = match c {
                    'n' => Some(Action::SortByName),
                    'e' => Some(Action::SortByExtension),
                    's' => Some(Action::SortBySize),
                    'w' => Some(Action::SortByWriteTime),
                    'c' => Some(Action::SortByCreationTime),
                    'a' => Some(Action::SortByAccessTime),
                    'd' => Some(Action::SortByDescription),
                    'o' => Some(Action::SortByOwner),
                    'u' => Some(Action::SortUnsorted),
                    'r' => Some(Action::ToggleSortReverse),
                    _ => None,
                };
                Ok(action)
            } else {
                Ok(None)
            }
        }
        Some(PopupType::YaziViewPopup) => {
            state.active_popup = None; // always close on keypress
            if let KeyCode::Char(c) = key.code {
                let action = match c {
                    '1' | 'b' => Some(Action::PanelViewBrief),
                    '2' | 'm' => Some(Action::PanelViewMedium),
                    '3' | 'f' => Some(Action::PanelViewFull),
                    '4' | 'w' => Some(Action::PanelViewWide),
                    '5' | 'd' => Some(Action::PanelViewDetailed),
                    '6' | 'x' => Some(Action::PanelViewDescriptions),
                    '7' | 'o' => Some(Action::PanelViewFileOwners),
                    '8' | 'l' => Some(Action::PanelViewFileLinks),
                    '9' | 'a' => Some(Action::PanelViewAltFull),
                    'i' => Some(Action::InfoPanel),
                    'q' => Some(Action::QuickView),
                    _ => None,
                };
                Ok(action)
            } else {
                Ok(None)
            }
        }
        _ => Err(()),
    }
}
