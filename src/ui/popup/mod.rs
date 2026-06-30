pub mod about;
pub mod color_groups;
pub mod config_dialog;
pub mod editor;
pub mod files_highlighting;
pub mod git_commit_prompt;
pub mod git_confirm_checkout;
pub mod git_panel;
pub mod history_lists;
pub mod info;
pub mod menus;
pub mod prompts;
pub mod screens_menu;
pub mod update;
pub mod viewer;
pub mod plugin_menu;
pub mod yazi;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
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
    let size = f.area();

    // If the active popup is a ScreensMenu that suspended another popup, render the suspended one first!
    if let PopupType::ScreensMenu {
        suspended_popup: Some(suspended),
        ..
    } = popup
    {
        render_specific_popup(
            f, suspended, state, context, left_rect, right_rect, theme, size,
        );
    }

    render_specific_popup(f, popup, state, context, left_rect, right_rect, theme, size);
}

fn render_specific_popup(
    f: &mut ratatui::Frame,
    popup: &PopupType,
    state: &AppState,
    context: &AppContext,
    left_rect: Rect,
    right_rect: Rect,
    theme: &crate::config::theme::Theme,
    size: Rect,
) {
    if prompts::render_prompt_popup(f, popup, theme, size, context) {
        return;
    }
    if yazi::render_yazi_popup(f, popup, theme, size) {
        return;
    }
    if menus::render_menu_popup(f, popup, theme, size, left_rect, right_rect, state, context) {
        return;
    }
    if screens_menu::render_screens_menu(f, popup, state, theme, size) {
        return;
    }
    if editor::render_editor_popup(f, popup, theme, size) {
        return;
    }
    if viewer::render_viewer_popup(f, popup, theme, size) {
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
    if about::render_about_popup(f, popup, theme, size) {
        return;
    }
    if plugin_menu::render(f, popup, theme, size, context) {
        return;
    }
    if git_panel::render(f, popup, theme, size) {
        return;
    }
    if git_commit_prompt::render(f, popup, theme, size) {
        return;
    }
    if git_confirm_checkout::render(f, popup, theme, size) {
        return;
    }
    if update::render(f, popup, theme, size) {
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

/// Centers a rectangle of fixed `width` and `height` over the full screen.
pub(crate) fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(r.height)),
            Constraint::Length(r.height.saturating_sub(height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(r.width)),
            Constraint::Length(r.width.saturating_sub(width) / 2),
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
