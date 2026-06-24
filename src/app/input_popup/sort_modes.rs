use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, SortField};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SortModesDialog {
        current,
        reverse,
        cursor_idx,
    }) = state.active_popup.clone()
    {
        let fields = [
            SortField::Name,
            SortField::Extension,
            SortField::Size,
            SortField::Date,
            SortField::Unsorted,
        ];
        let total_items = fields.len() + 1; // fields + reverse order option

        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Up => {
                let new_idx = if cursor_idx > 0 {
                    cursor_idx - 1
                } else {
                    total_items - 1
                };
                state.active_popup = Some(PopupType::SortModesDialog {
                    current,
                    reverse,
                    cursor_idx: new_idx,
                });
                return Ok(None);
            }
            KeyCode::Down => {
                let new_idx = if cursor_idx < total_items - 1 {
                    cursor_idx + 1
                } else {
                    0
                };
                state.active_popup = Some(PopupType::SortModesDialog {
                    current,
                    reverse,
                    cursor_idx: new_idx,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                if cursor_idx < fields.len() {
                    let chosen_field = fields[cursor_idx];
                    let panel = state.get_active_panel_mut();
                    panel.sort_field = chosen_field;
                    panel.sort_reverse = reverse;
                } else {
                    let panel = state.get_active_panel_mut();
                    panel.sort_reverse = !reverse;
                }
                state.active_popup = None;
                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(None);
            }
            KeyCode::Char(' ') => {
                if cursor_idx < fields.len() {
                    let chosen_field = fields[cursor_idx];
                    let panel = state.get_active_panel_mut();
                    panel.sort_field = chosen_field;
                    state.active_popup = Some(PopupType::SortModesDialog {
                        current: chosen_field,
                        reverse,
                        cursor_idx,
                    });
                } else {
                    let panel = state.get_active_panel_mut();
                    panel.sort_reverse = !reverse;
                    state.active_popup = Some(PopupType::SortModesDialog {
                        current,
                        reverse: !reverse,
                        cursor_idx,
                    });
                }
                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
