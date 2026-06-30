use super::git;
use super::rows;
use crate::app::state::PopupType;
use crate::config::settings::Settings;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_editing(
    key: KeyEvent,
    active_tab: usize,
    cursor_idx: usize,
    mut editing_value: bool,
    mut edit_buffer: String,
    mut settings: Settings,
    focus_on_tabs: bool,
) -> Option<PopupType> {
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if editing_value {
        match key.code {
            KeyCode::Char(c) if !is_ctrl => {
                edit_buffer.push(c);
            }
            KeyCode::Backspace => {
                edit_buffer.pop();
            }
            KeyCode::Esc => {
                editing_value = false;
            }
            KeyCode::Enter => {
                let current_rows = rows::get_rows_for_tab(
                    active_tab,
                    &settings,
                    editing_value,
                    cursor_idx,
                    &edit_buffer,
                );
                // Extract setting ID
                if cursor_idx < current_rows.len() {
                    if let crate::ui::popup::config_dialog::RowType::Setting(setting_id) =
                        current_rows[cursor_idx].1
                    {
                        if active_tab == 5 && setting_id == 1 {
                            settings.default_editor = edit_buffer.clone();
                        } else if active_tab == 5 && setting_id == 22 {
                            settings.viewer_command = edit_buffer.clone();
                        } else if active_tab == 2 && setting_id == 14 {
                            settings.interface_window_title_addons = edit_buffer.clone();
                        } else if active_tab == 4 && setting_id == 12 {
                            settings.plugins_dev_dir = edit_buffer.clone();
                        } else if active_tab == 7 {
                            git::apply_edit(setting_id, &mut settings, &edit_buffer);
                        }
                    }
                }
                editing_value = false;
            }
            _ => {}
        }
    }

    Some(PopupType::ConfigurationDialog {
        active_tab,
        cursor_idx,
        editing_value,
        edit_buffer,
        settings,
        focus_on_tabs,
    })
}
