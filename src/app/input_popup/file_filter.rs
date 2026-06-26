use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(popup) = state.active_popup.clone() {
        match popup {
            PopupType::FilePanelFilterPrompt { input } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input;
                        new_input.push(c);
                        state.active_popup =
                            Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input;
                        new_input.pop();
                        state.active_popup =
                            Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let mask = input.trim().to_string();
                        state.active_popup = None;
                        let panel = state.get_active_panel_mut();
                        panel.filter_mask = if mask.is_empty() { None } else { Some(mask) };
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            PopupType::QuickFilterPrompt {
                input,
                original_mask,
                original_cursor,
            } => {
                let active_panel = state.active_panel;
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input;
                        new_input.push(c);
                        state.active_popup = Some(PopupType::QuickFilterPrompt {
                            input: new_input.clone(),
                            original_mask,
                            original_cursor,
                        });
                        state.update_panel_filter(active_panel, Some(new_input));
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input;
                        new_input.pop();
                        state.active_popup = Some(PopupType::QuickFilterPrompt {
                            input: new_input.clone(),
                            original_mask,
                            original_cursor,
                        });
                        state.update_panel_filter(active_panel, Some(new_input));
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        state.update_panel_filter(active_panel, original_mask);
                        let panel = match active_panel {
                            crate::app::state::ActivePanel::Left => &mut state.left_panel,
                            crate::app::state::ActivePanel::Right => &mut state.right_panel,
                        };
                        panel.cursor_index = original_cursor;
                        return Ok(None);
                    }
                    _ => {}
                }
                Err(())
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
