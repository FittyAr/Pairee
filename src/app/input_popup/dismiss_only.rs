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
            | PopupType::CompareFoldersResult { .. }
            | PopupType::FileAssociationsDialog { .. } => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    state.active_popup = None;
                    return Ok(None);
                }
                Err(())
            }
            PopupType::QuickViewPanel {
                path,
                content,
                scroll,
                image_data,
            } => {
                if key.code == KeyCode::Esc {
                    state.active_popup = None;
                    state.quick_view_active = false;
                    return Ok(None);
                }
                if key.code == KeyCode::PageDown {
                    let max_scroll = if let Some(ref img) = image_data {
                        let rows = (img.height() as usize + 1) / 2;
                        rows.saturating_sub(5)
                    } else {
                        let visible_height = 20;
                        content.len().saturating_sub(visible_height)
                    };
                    let new_scroll = (scroll + 15).min(max_scroll);
                    state.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content,
                        scroll: new_scroll,
                        image_data,
                    });
                    return Ok(None);
                }
                if key.code == KeyCode::PageUp {
                    let new_scroll = scroll.saturating_sub(15);
                    state.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content,
                        scroll: new_scroll,
                        image_data,
                    });
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
