pub mod commands;
pub mod files;
pub mod options;
pub mod panel;
pub mod panel_submenus;
pub mod panel_tabs;
pub mod types;

pub use types::MenuItemData;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::Menu;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
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
    use crate::app::state::ActivePanel::{Left, Right};
    match menu_idx {
        0 => panel::get_items(state, resolver, settings, Left),
        1 => files::get_items(&state.journal, resolver, settings),
        2 => commands::get_items(resolver, settings),
        3 => options::get_items(resolver),
        4 => panel::get_items(state, resolver, settings, Right),
        5 => panel_submenus::view_items(state, resolver, Left),
        6 => panel_submenus::sort_items(state, resolver, Left),
        7 => panel_submenus::view_items(state, resolver, Right),
        8 => panel_submenus::sort_items(state, resolver, Right),
        9 => panel_tabs::tab_items(state, resolver, Left),
        10 => panel_tabs::tab_items(state, resolver, Right),
        _ => vec![],
    }
}

/// The panel a Left / Right menu (or one of their submenus) acts on.
pub fn menu_side(menu_idx: usize) -> Option<crate::app::state::ActivePanel> {
    use crate::app::state::ActivePanel::{Left, Right};
    match menu_idx {
        0 | 5 | 6 | 9 => Some(Left),
        4 | 7 | 8 | 10 => Some(Right),
        _ => None,
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

/// Screen cells of the top menu titles on a bar starting at `area`, in
/// title order (the drop-downs open below them, clicks find them).
pub fn title_cells(area: Rect) -> Vec<Rect> {
    let mut x = area.x;
    get_menu_titles()
        .iter()
        .map(|title| {
            let width = crate::ui::text_width::display_width(
                &crate::ui::hotkey::parse_hotkey(title).clean_text,
            ) as u16;
            let cell = Rect::new(x, area.y, width, 1).intersection(area);
            x = x.saturating_add(width);
            cell
        })
        .collect()
}

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    for (i, cell) in title_cells(area).into_iter().enumerate() {
        crate::ui::hit::register(cell, crate::ui::hit::HitTarget::MenuTitle(i));
    }
    let theme = &context.theme();

    let active_menu_idx = if let Some(Menu {
        active_menu_idx, ..
    }) = state.dialogs.top_as::<Menu>()
    {
        Some(active_menu_idx)
    } else {
        None
    };

    let mut spans = Vec::new();
    let titles = get_menu_titles();
    for (i, title) in titles.iter().enumerate() {
        let is_active = Some(i) == active_menu_idx.copied();
        let style = if is_active {
            theme.selection().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.panel_fg)
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
    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, menu_chunks[0]);

    if state.is_root {
        let root_para = Paragraph::new(" [ROOT] ").style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(ratatui::style::Color::Red)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(root_para, menu_chunks[1]);
    }

    if context.config.settings.interface_clock {
        let time_str = chrono::Local::now().format(" %H:%M:%S ").to_string();
        let clock_para = Paragraph::new(time_str)
            .style(Style::default().bg(Color::DarkGray).fg(theme.panel_fg))
            .alignment(ratatui::layout::Alignment::Right);
        f.render_widget(clock_para, menu_chunks[2]);
    } else {
        let empty_para = Paragraph::new("").style(Style::default().bg(Color::DarkGray));
        f.render_widget(empty_para, menu_chunks[2]);
    }
}
