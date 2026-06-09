pub mod colors;
pub mod confirmations;
pub mod editor_viewer;
pub mod interface;
pub mod panel;
pub mod plugins;
pub mod system;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::{change_preset, change_theme};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ConfigurationDialog {
        mut active_tab,
        mut cursor_idx,
        mut editing_value,
        mut edit_buffer,
        mut settings,
    }) = state.active_popup.clone()
    {
        let max_rows = match active_tab {
            0 => 19, // System (17 settings + 2 buttons)
            1 => 35, // Panel (33 settings + 2 buttons)
            2 => 39, // Interface (37 settings + 2 buttons)
            3 => 16, // Confirmations (14 settings + 2 buttons)
            4 => 13, // Language & Plugins (11 settings + 2 buttons)
            5 => 41, // Editor/Viewer (39 settings + 2 buttons)
            6 => 5,  // Colors (3 settings + 2 buttons)
            _ => 5,
        };

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
                    if active_tab == 5 && cursor_idx == 1 {
                        settings.default_editor = edit_buffer.clone();
                    } else if active_tab == 5 && cursor_idx == 22 {
                        settings.viewer_command = edit_buffer.clone();
                    } else if active_tab == 2 && cursor_idx == 14 {
                        settings.interface_window_title_addons = edit_buffer.clone();
                    }
                    editing_value = false;
                }
                _ => {}
            }
            state.active_popup = Some(PopupType::ConfigurationDialog {
                active_tab,
                cursor_idx,
                editing_value,
                edit_buffer,
                settings,
            });
            return Ok(None);
        }

        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Left => {
                if active_tab > 0 {
                    active_tab -= 1;
                } else {
                    active_tab = 6;
                }
                cursor_idx = 0;
            }
            KeyCode::Right => {
                if active_tab < 6 {
                    active_tab += 1;
                } else {
                    active_tab = 0;
                }
                cursor_idx = 0;
            }
            KeyCode::Up => {
                if cursor_idx > 0 {
                    cursor_idx -= 1;
                } else {
                    cursor_idx = max_rows - 1;
                }
            }
            KeyCode::Down => {
                if cursor_idx < max_rows - 1 {
                    cursor_idx += 1;
                } else {
                    cursor_idx = 0;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                let ok_idx = max_rows - 2;
                let cancel_idx = max_rows - 1;

                if cursor_idx == ok_idx {
                    if settings.theme != context.config.settings.theme {
                        change_theme(context, state, &settings.theme);
                    }
                    if settings.keybinding_preset != context.config.settings.keybinding_preset {
                        change_preset(context, &settings.keybinding_preset);
                    }
                    state.case_sensitive_sort = settings.case_sensitive_sort;
                    state.treat_digits_as_numbers = settings.treat_digits_as_numbers;
                    state.sorting_collation = settings.sorting_collation.clone();
                    state.req_admin_reading = settings.req_admin_reading;
                    // Panel settings
                    state.select_folders = settings.select_folders;
                    state.sort_folder_names_by_extension = settings.sort_folder_names_by_extension;
                    state.show_dotdot_in_root_folders = settings.show_dotdot_in_root_folders;
                    state.disable_panel_update_object_count =
                        settings.disable_panel_update_object_count;
                    let lang_to_load = settings.language.clone();
                    context.config.settings = settings;
                    let _ = context.config.save();
                    crate::config::localization::load_language(&lang_to_load);
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    state.active_popup = None;
                    return Ok(None);
                } else if cursor_idx == cancel_idx {
                    state.active_popup = None;
                    return Ok(None);
                }

                let next_popup = match active_tab {
                    0 => system::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    1 => panel::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    2 => interface::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    3 => confirmations::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    4 => plugins::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    5 => editor_viewer::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    6 => colors::handle_row(
                        cursor_idx,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                        context,
                    ),
                    _ => None,
                };
                
                if let Some(popup) = next_popup {
                    // Temporarily save settings inside context so they aren't lost
                    // Wait, we need to save the current settings into context!
                    // Let's just set the popup and return.
                    state.active_popup = Some(popup);
                    return Ok(None);
                }
            }
            KeyCode::F(9) => {
                if settings.theme != context.config.settings.theme {
                    change_theme(context, state, &settings.theme);
                }
                if settings.keybinding_preset != context.config.settings.keybinding_preset {
                    change_preset(context, &settings.keybinding_preset);
                }
                state.case_sensitive_sort = settings.case_sensitive_sort;
                state.treat_digits_as_numbers = settings.treat_digits_as_numbers;
                state.sorting_collation = settings.sorting_collation.clone();
                state.req_admin_reading = settings.req_admin_reading;
                // Panel settings
                state.select_folders = settings.select_folders;
                state.sort_folder_names_by_extension = settings.sort_folder_names_by_extension;
                state.show_dotdot_in_root_folders = settings.show_dotdot_in_root_folders;
                state.disable_panel_update_object_count =
                    settings.disable_panel_update_object_count;
                let lang_to_load = settings.language.clone();
                context.config.settings = settings;
                let _ = context.config.save();
                crate::config::localization::load_language(&lang_to_load);
                state.refresh_both_panels(context.config.settings.show_hidden);
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char(c) => {
                let lower_c = c.to_ascii_lowercase();
                let tab_titles = [
                    crate::config::localization::t("tab_system"),
                    crate::config::localization::t("tab_panel"),
                    crate::config::localization::t("tab_interface"),
                    crate::config::localization::t("tab_confirmations"),
                    crate::config::localization::t("tab_plugins"),
                    crate::config::localization::t("tab_editor"),
                    crate::config::localization::t("tab_colors"),
                ];
                for (i, title) in tab_titles.iter().enumerate() {
                    let parsed = crate::ui::hotkey::parse_hotkey(&title);
                    if let Some(hotkey) = parsed.hotkey {
                        if hotkey == lower_c {
                            active_tab = i;
                            cursor_idx = 0;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }

        state.active_popup = Some(PopupType::ConfigurationDialog {
            active_tab,
            cursor_idx,
            editing_value,
            edit_buffer,
            settings,
        });
        return Ok(None);
    }
    Err(())
}
