use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub const COLORS: [&str; 17] = [
    "Reset",
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "Gray",
    "DarkGray",
    "LightRed",
    "LightGreen",
    "LightYellow",
    "LightBlue",
    "LightMagenta",
    "LightCyan",
    "White",
];

fn cycle_color(current: &String, direction: i32) -> String {
    let mut idx = COLORS.iter().position(|&c| c == current).unwrap_or(0) as i32;
    idx += direction;
    if idx < 0 {
        idx = (COLORS.len() as i32) - 1;
    } else if idx >= COLORS.len() as i32 {
        idx = 0;
    }
    COLORS[idx as usize].to_string()
}

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ColorGroupsDialog {
        mut cursor_idx,
        mut editing,
        mut edit_buffer,
        mut theme,
    }) = state.active_popup.clone()
    {
        if editing {
            match key.code {
                KeyCode::Esc => {
                    editing = false;
                }
                KeyCode::Enter => {
                    editing = false;
                    let val = edit_buffer.clone();
                    match cursor_idx {
                        0 => theme.panel_bg = val,
                        1 => theme.panel_fg = val,
                        2 => theme.panel_border = val,
                        3 => theme.selection_bg = val,
                        4 => theme.selection_fg = val,
                        5 => theme.marked_fg = val,
                        6 => theme.header_bg = val,
                        7 => theme.header_fg = val,
                        8 => theme.cli_bg = val,
                        9 => theme.cli_fg = val,
                        10 => theme.fkey_num_fg = val,
                        11 => theme.fkey_text_fg = val,
                        12 => theme.fkey_bg = val,
                        13 => theme.popup_bg = val,
                        14 => theme.popup_fg = val,
                        15 => theme.popup_border = val,
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    edit_buffer.push(c);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    // Return to Configuration Dialog
                    state.active_popup = Some(PopupType::ConfigurationDialog {
                        active_tab: 6, // Colors tab
                        cursor_idx: 1, // Color groups
                        editing_value: false,
                        edit_buffer: String::new(),
                        settings: context.config.settings.clone(),
                        focus_on_tabs: false,
                    });
                    return Ok(None);
                }
                KeyCode::Enter => {
                    let prop_value = match cursor_idx {
                        0 => &theme.panel_bg,
                        1 => &theme.panel_fg,
                        2 => &theme.panel_border,
                        3 => &theme.selection_bg,
                        4 => &theme.selection_fg,
                        5 => &theme.marked_fg,
                        6 => &theme.header_bg,
                        7 => &theme.header_fg,
                        8 => &theme.cli_bg,
                        9 => &theme.cli_fg,
                        10 => &theme.fkey_num_fg,
                        11 => &theme.fkey_text_fg,
                        12 => &theme.fkey_bg,
                        13 => &theme.popup_bg,
                        14 => &theme.popup_fg,
                        15 => &theme.popup_border,
                        _ => &theme.panel_bg,
                    };
                    editing = true;
                    edit_buffer = prop_value.clone();
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Quick save shortcut if needed, or wait, Enter is used to edit!
                    // Let's make "S" save and return.
                    if context.config.settings.theme == "slate"
                        || context.config.settings.theme == "classic_blue"
                    {
                        context.config.settings.theme = "custom".to_string();
                    }
                    context.config.theme = theme.clone();
                    let _ = context.config.save(); // Save configuration
                    state.refresh_both_panels(context.config.settings.show_hidden);

                    // Return to Configuration Dialog
                    state.active_popup = Some(PopupType::ConfigurationDialog {
                        active_tab: 6,
                        cursor_idx: 1,
                        editing_value: false,
                        edit_buffer: String::new(),
                        settings: context.config.settings.clone(),
                        focus_on_tabs: false,
                    });
                    return Ok(None);
                }
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = crate::ui::popup::color_groups::THEME_PROPS.len() - 1;
                    }
                }
                KeyCode::Down => {
                    if cursor_idx < crate::ui::popup::color_groups::THEME_PROPS.len() - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Left => {
                    let dir = -1;
                    match cursor_idx {
                        0 => theme.panel_bg = cycle_color(&theme.panel_bg, dir),
                        1 => theme.panel_fg = cycle_color(&theme.panel_fg, dir),
                        2 => theme.panel_border = cycle_color(&theme.panel_border, dir),
                        3 => theme.selection_bg = cycle_color(&theme.selection_bg, dir),
                        4 => theme.selection_fg = cycle_color(&theme.selection_fg, dir),
                        5 => theme.marked_fg = cycle_color(&theme.marked_fg, dir),
                        6 => theme.header_bg = cycle_color(&theme.header_bg, dir),
                        7 => theme.header_fg = cycle_color(&theme.header_fg, dir),
                        8 => theme.cli_bg = cycle_color(&theme.cli_bg, dir),
                        9 => theme.cli_fg = cycle_color(&theme.cli_fg, dir),
                        10 => theme.fkey_num_fg = cycle_color(&theme.fkey_num_fg, dir),
                        11 => theme.fkey_text_fg = cycle_color(&theme.fkey_text_fg, dir),
                        12 => theme.fkey_bg = cycle_color(&theme.fkey_bg, dir),
                        13 => theme.popup_bg = cycle_color(&theme.popup_bg, dir),
                        14 => theme.popup_fg = cycle_color(&theme.popup_fg, dir),
                        15 => theme.popup_border = cycle_color(&theme.popup_border, dir),
                        _ => {}
                    }
                }
                KeyCode::Right => {
                    let dir = 1;
                    match cursor_idx {
                        0 => theme.panel_bg = cycle_color(&theme.panel_bg, dir),
                        1 => theme.panel_fg = cycle_color(&theme.panel_fg, dir),
                        2 => theme.panel_border = cycle_color(&theme.panel_border, dir),
                        3 => theme.selection_bg = cycle_color(&theme.selection_bg, dir),
                        4 => theme.selection_fg = cycle_color(&theme.selection_fg, dir),
                        5 => theme.marked_fg = cycle_color(&theme.marked_fg, dir),
                        6 => theme.header_bg = cycle_color(&theme.header_bg, dir),
                        7 => theme.header_fg = cycle_color(&theme.header_fg, dir),
                        8 => theme.cli_bg = cycle_color(&theme.cli_bg, dir),
                        9 => theme.cli_fg = cycle_color(&theme.cli_fg, dir),
                        10 => theme.fkey_num_fg = cycle_color(&theme.fkey_num_fg, dir),
                        11 => theme.fkey_text_fg = cycle_color(&theme.fkey_text_fg, dir),
                        12 => theme.fkey_bg = cycle_color(&theme.fkey_bg, dir),
                        13 => theme.popup_bg = cycle_color(&theme.popup_bg, dir),
                        14 => theme.popup_fg = cycle_color(&theme.popup_fg, dir),
                        15 => theme.popup_border = cycle_color(&theme.popup_border, dir),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        state.active_popup = Some(PopupType::ColorGroupsDialog {
            cursor_idx,
            editing,
            edit_buffer,
            theme,
        });
        return Ok(None);
    }
    Err(())
}
