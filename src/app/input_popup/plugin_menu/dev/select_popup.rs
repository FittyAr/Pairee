//! Key-event handler for the "Select active development plugin" modal.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_select_popup(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let (options, mut cursor_idx, previous_popup) = match state.active_popup.clone() {
        Some(PopupType::SelectDevPlugin {
            options,
            cursor_idx,
            previous_popup,
        }) => (options, cursor_idx, previous_popup),
        _ => return Err(()),
    };

    match key.code {
        KeyCode::Esc => {
            state.active_popup = Some(*previous_popup);
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if !options.is_empty() {
                if cursor_idx == 0 {
                    cursor_idx = options.len() - 1;
                } else {
                    cursor_idx -= 1;
                }
            }
            state.active_popup = Some(PopupType::SelectDevPlugin {
                options,
                cursor_idx,
                previous_popup,
            });
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !options.is_empty() {
                if cursor_idx >= options.len() - 1 {
                    cursor_idx = 0;
                } else {
                    cursor_idx += 1;
                }
            }
            state.active_popup = Some(PopupType::SelectDevPlugin {
                options,
                cursor_idx,
                previous_popup,
            });
        }
        KeyCode::Enter => {
            if cursor_idx < options.len() {
                let (_, value) = &options[cursor_idx];
                if value.is_empty() || value == "deselect" {
                    context.config.settings.active_dev_plugin = None;
                    let _ = context.config.save();
                } else {
                    context.config.settings.active_dev_plugin = Some(value.clone());
                    let _ = context.config.save();
                }
            }

            let mut prev = *previous_popup;
            if let PopupType::PluginMenu {
                ref mut installed,
                ref mut dev_results,
                ..
            } = prev
            {
                *installed = super::reload_installed_plugins(context, &None);
                if let Some(ref active) = context.config.settings.active_dev_plugin {
                    *dev_results = t("plugin_dev_selected").replace("{}", active);
                } else {
                    *dev_results = t("plugin_dev_deselected");
                }
            }
            state.active_popup = Some(prev);
        }
        _ => {}
    }

    Ok(None)
}
