use super::{colors, confirmations, editor_viewer, git, interface, panel, plugins, rows, system};
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::settings::Settings;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_navigation(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
    mut active_tab: usize,
    mut cursor_idx: usize,
    mut editing_value: bool,
    mut edit_buffer: String,
    mut settings: Settings,
    mut focus_on_tabs: bool,
) -> Result<Option<Action>, ()> {
    let mut current_rows = rows::get_rows_for_tab(
        active_tab,
        &settings,
        editing_value,
        cursor_idx,
        &edit_buffer,
    );
    let max_rows = current_rows.len() + 2;

    // Ensure initial cursor_idx is selectable
    if !rows::is_selectable(cursor_idx, &current_rows) {
        while cursor_idx < max_rows && !rows::is_selectable(cursor_idx, &current_rows) {
            cursor_idx += 1;
        }
        if cursor_idx >= max_rows {
            cursor_idx = 0;
        }
    }

    match key.code {
        KeyCode::Esc => {
            state.active_popup = None;
            return Ok(None);
        }
        KeyCode::Tab => {
            focus_on_tabs = !focus_on_tabs;
        }
        KeyCode::BackTab => {
            focus_on_tabs = true;
        }
        KeyCode::Left => {
            if !focus_on_tabs {
                focus_on_tabs = true;
            }
        }
        KeyCode::Right => {
            if focus_on_tabs {
                focus_on_tabs = false;
            }
        }
        KeyCode::Up => {
            if focus_on_tabs {
                if active_tab > 0 {
                    active_tab -= 1;
                } else {
                    active_tab = 7;
                }
                cursor_idx = 0;
                current_rows = rows::get_rows_for_tab(
                    active_tab,
                    &settings,
                    editing_value,
                    cursor_idx,
                    &edit_buffer,
                );
                while cursor_idx < current_rows.len()
                    && !rows::is_selectable(cursor_idx, &current_rows)
                {
                    cursor_idx += 1;
                }
            } else {
                loop {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = max_rows - 1;
                    }
                    if rows::is_selectable(cursor_idx, &current_rows) {
                        break;
                    }
                }
            }
        }
        KeyCode::Down => {
            if focus_on_tabs {
                if active_tab < 7 {
                    active_tab += 1;
                } else {
                    active_tab = 0;
                }
                cursor_idx = 0;
                current_rows = rows::get_rows_for_tab(
                    active_tab,
                    &settings,
                    editing_value,
                    cursor_idx,
                    &edit_buffer,
                );
                while cursor_idx < current_rows.len()
                    && !rows::is_selectable(cursor_idx, &current_rows)
                {
                    cursor_idx += 1;
                }
            } else {
                loop {
                    if cursor_idx < max_rows - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                    if rows::is_selectable(cursor_idx, &current_rows) {
                        break;
                    }
                }
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if focus_on_tabs {
                focus_on_tabs = false;
            } else {
                let ok_idx = max_rows - 2;
                let cancel_idx = max_rows - 1;

                if cursor_idx == ok_idx {
                    super::apply::apply_settings(state, context, settings);
                    return Ok(None);
                } else if cursor_idx == cancel_idx {
                    state.active_popup = None;
                    return Ok(None);
                }

                // Map visual cursor_idx to actual setting ID
                let setting_id = if cursor_idx < current_rows.len() {
                    match current_rows[cursor_idx].1 {
                        crate::ui::popup::config_dialog::RowType::Setting(id) => id,
                        _ => return Ok(None),
                    }
                } else {
                    return Ok(None);
                };

                let next_popup = match active_tab {
                    0 => system::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    1 => panel::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    2 => interface::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    3 => confirmations::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    4 => plugins::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    5 => editor_viewer::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    6 => colors::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                        context,
                    ),
                    7 => git::handle_row(
                        setting_id,
                        &mut settings,
                        &mut editing_value,
                        &mut edit_buffer,
                    ),
                    _ => None,
                };

                if let Some(popup) = next_popup {
                    state.active_popup = Some(popup);
                    return Ok(None);
                }
            }
        }
        KeyCode::F(9) => {
            super::apply::apply_settings(state, context, settings);
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
                crate::config::localization::t("tab_git"),
            ];
            for (i, title) in tab_titles.iter().enumerate() {
                let parsed = crate::ui::hotkey::parse_hotkey(&title);
                if let Some(hotkey) = parsed.hotkey {
                    if hotkey == lower_c {
                        active_tab = i;
                        cursor_idx = 0;
                        focus_on_tabs = false;
                        current_rows = rows::get_rows_for_tab(
                            active_tab,
                            &settings,
                            editing_value,
                            cursor_idx,
                            &edit_buffer,
                        );
                        while cursor_idx < current_rows.len()
                            && !rows::is_selectable(cursor_idx, &current_rows)
                        {
                            cursor_idx += 1;
                        }
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
        focus_on_tabs,
    });
    Ok(None)
}
