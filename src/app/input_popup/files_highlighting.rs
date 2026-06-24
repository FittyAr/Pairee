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
    if let Some(PopupType::FilesHighlightingDialog {
        mut cursor_idx,
        mut editing,
        mut edit_buffer,
        mut rules,
    }) = state.active_popup.clone()
    {
        if editing {
            match key.code {
                KeyCode::Esc => {
                    editing = false;
                }
                KeyCode::Enter => {
                    editing = false;
                    rules[cursor_idx].color = edit_buffer.clone();
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
                        cursor_idx: 2, // Files highlighting
                        editing_value: false,
                        edit_buffer: String::new(),
                        settings: context.config.settings.clone(),
                        focus_on_tabs: false,
                    });
                    return Ok(None);
                }
                KeyCode::Enter => {
                    if !rules.is_empty() {
                        editing = true;
                        edit_buffer = rules[cursor_idx].color.clone();
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Apply rules to context
                    context.config.settings.highlight_rules = rules.clone();
                    let _ = context.config.save(); // Save configuration
                    state.refresh_both_panels(context.config.settings.show_hidden);

                    // Return to Configuration Dialog
                    state.active_popup = Some(PopupType::ConfigurationDialog {
                        active_tab: 6,
                        cursor_idx: 2,
                        editing_value: false,
                        edit_buffer: String::new(),
                        settings: context.config.settings.clone(),
                        focus_on_tabs: false,
                    });
                    return Ok(None);
                }
                KeyCode::Up => {
                    if rules.is_empty() {
                        return Ok(None);
                    }
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = rules.len() - 1;
                    }
                }
                KeyCode::Down => {
                    if rules.is_empty() {
                        return Ok(None);
                    }
                    if cursor_idx < rules.len() - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Left => {
                    if !rules.is_empty() {
                        rules[cursor_idx].color = cycle_color(&rules[cursor_idx].color, -1);
                    }
                }
                KeyCode::Right => {
                    if !rules.is_empty() {
                        rules[cursor_idx].color = cycle_color(&rules[cursor_idx].color, 1);
                    }
                }
                _ => {}
            }
        }
        state.active_popup = Some(PopupType::FilesHighlightingDialog {
            cursor_idx,
            editing,
            edit_buffer,
            rules,
        });
        return Ok(None);
    }
    Err(())
}
