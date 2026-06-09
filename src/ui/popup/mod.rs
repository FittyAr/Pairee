pub mod config_dialog;
pub mod color_groups;
pub mod editor;
pub mod files_highlighting;
pub mod history_lists;
pub mod info;
pub mod menus;
pub mod prompts;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub fn render_popup(
    f: &mut Frame,
    state: &AppState,
    context: &AppContext,
    left_rect: Rect,
    right_rect: Rect,
) {
    let popup = match &state.active_popup {
        Some(p) => p,
        None => return,
    };

    let theme = &context.config.theme;
    let size = f.size();

    if prompts::render_prompt_popup(f, popup, theme, size) {
        return;
    }
    if menus::render_menu_popup(
        f,
        popup,
        theme,
        size,
        left_rect,
        right_rect,
        state,
    ) {
        return;
    }
    if editor::render_editor_popup(f, popup, theme, size) {
        return;
    }
    if history_lists::render_history_lists_popup(f, popup, theme, size) {
        return;
    }
    if config_dialog::render_config_dialog_popup(f, popup, theme, size) {
        return;
    }
    if color_groups::render_color_groups_popup(f, popup, theme, size) {
        return;
    }
    if files_highlighting::render_files_highlighting_popup(f, popup, theme, size) {
        return;
    }
    if info::render_info_popup(f, popup, theme, size) {
        return;
    }
}

/// Centers a rectangle of `percent_x` × `percent_y` over the full screen.
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Centers a rectangle of `percent_x` × `percent_y` within a given parent rectangle.
/// Used for panel-specific popups (e.g. DriveSelect).
pub(crate) fn centered_rect_in(percent_x: u16, percent_y: u16, parent: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(parent);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
