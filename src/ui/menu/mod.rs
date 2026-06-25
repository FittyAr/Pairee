pub mod commands;
pub mod files;
pub mod left;
pub mod left_sort;
pub mod left_view;
pub mod options;
pub mod right;
pub mod right_sort;
pub mod right_view;
pub mod types;

pub use types::MenuItemData;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

/// Returns the menu item labels for a given top-level menu index.
pub fn get_menu_items(
    menu_idx: usize,
    state: &AppState,
    resolver: &crate::keybindings::KeybindingResolver,
    settings: &crate::config::settings::Settings,
) -> Vec<MenuItemData> {
    match menu_idx {
        0 => left::get_items(state, resolver, settings),
        1 => files::get_items(resolver),
        2 => commands::get_items(resolver),
        3 => options::get_items(resolver),
        4 => right::get_items(state, resolver, settings),
        5 => left_view::get_items(state, resolver),
        6 => left_sort::get_items(state, resolver),
        7 => right_view::get_items(state, resolver),
        8 => right_sort::get_items(state, resolver),
        _ => vec![],
    }
}

/// Returns the display labels for the top-level menu bar.
pub fn get_menu_titles() -> Vec<String> {
    vec![
        format!("  {}  ", t("menu_left")),
        format!("  {}  ", t("menu_files")),
        format!("  {}  ", t("menu_commands")),
        format!("  {}  ", t("menu_options")),
        format!("  {}  ", t("menu_right")),
    ]
}

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let active_menu_idx = if let Some(PopupType::Menu {
        active_menu_idx, ..
    }) = state.active_popup
    {
        Some(active_menu_idx)
    } else {
        None
    };

    let mut spans = Vec::new();
    let titles = get_menu_titles();
    for (i, title) in titles.iter().enumerate() {
        let is_active = Some(i) == active_menu_idx;
        let style = if is_active {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(parse_color(&theme.panel_fg))
                .add_modifier(Modifier::BOLD)
        };

        let hotkey_style = style.fg(ratatui::style::Color::Yellow);
        let hotkey_spans = crate::ui::hotkey::render_hotkey_spans(title, style, hotkey_style);
        spans.extend(hotkey_spans);
    }

    let menu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(if state.is_root { 8 } else { 0 }),
            Constraint::Length(12),
        ])
        .split(area);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color("DarkGray")));
    f.render_widget(paragraph, menu_chunks[0]);

    if state.is_root {
        let root_para = Paragraph::new(" [ROOT] ").style(
            Style::default()
                .bg(parse_color("DarkGray"))
                .fg(ratatui::style::Color::Red)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(root_para, menu_chunks[1]);
    }

    if context.config.settings.interface_clock {
        let time_str = chrono::Local::now().format(" %H:%M:%S ").to_string();
        let clock_para = Paragraph::new(time_str)
            .style(
                Style::default()
                    .bg(parse_color("DarkGray"))
                    .fg(parse_color(&theme.panel_fg)),
            )
            .alignment(ratatui::layout::Alignment::Right);
        f.render_widget(clock_para, menu_chunks[2]);
    } else {
        let empty_para = Paragraph::new("").style(Style::default().bg(parse_color("DarkGray")));
        f.render_widget(empty_para, menu_chunks[2]);
    }
}
