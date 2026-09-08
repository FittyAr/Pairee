//! Rendering of the top navigation dropdown menu and cascading submenus.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_top_dropdown(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    state: &AppState,
    context: &AppContext,
    active_menu_idx: usize,
    active_item_idx: Option<usize>,
    active_submenu_idx: Option<usize>,
    active_submenu_item_idx: Option<usize>,
) {
    let active_item_idx = match active_item_idx {
        Some(idx) => idx,
        None => return, // Only top menu bar is active, no dropdown to render
    };

    let items = crate::ui::menu::get_menu_items(
        active_menu_idx,
        state,
        &context.resolver,
        &context.config.settings,
    );
    let dropdown_x = match active_menu_idx {
        0 => 2,
        1 => 10,
        2 => 19,
        3 => 31,
        4 => 42,
        _ => 2,
    };
    let dropdown_width = 37;
    let dropdown_height = (items.len() + 2) as u16;
    let dropdown_rect =
        Rect::new(dropdown_x, 1, dropdown_width, dropdown_height).intersection(size);

    f.render_widget(Clear, dropdown_rect);

    let mut lines = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let is_cursor = i == active_item_idx && active_submenu_idx.is_none();
        let is_submenu_parent = i == active_item_idx && active_submenu_idx.is_some();

        let base_style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else if is_submenu_parent {
            Style::default()
                .bg(parse_color("Blue"))
                .fg(parse_color("White"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };

        if item.is_separator {
            lines.push(Line::from(Span::styled(
                " ───────────────────────────────── ",
                base_style,
            )));
            continue;
        }

        let hotkey_style = if is_cursor {
            base_style.fg(ratatui::style::Color::Yellow)
        } else {
            base_style
                .fg(ratatui::style::Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        let mut line_spans = Vec::new();
        let active_char = if item.active { "•" } else { " " };
        line_spans.push(Span::styled(format!(" {} ", active_char), base_style));

        let parsed = crate::ui::hotkey::parse_hotkey(&item.label);
        let label_spans =
            crate::ui::hotkey::render_hotkey_spans(&item.label, base_style, hotkey_style);
        line_spans.extend(label_spans);

        let label_len = parsed.clean_text.chars().count();
        let padding = 25usize.saturating_sub(label_len);
        line_spans.push(Span::styled(" ".repeat(padding), base_style));

        line_spans.push(Span::styled(format!("{:<7}", item.shortcut), base_style));

        lines.push(Line::from(line_spans));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .style(Style::default().bg(parse_color(&theme.popup_bg))),
    );

    f.render_widget(paragraph, dropdown_rect);

    if let (Some(sub_idx), Some(sub_item_idx)) = (active_submenu_idx, active_submenu_item_idx) {
        let sub_items = crate::ui::menu::get_menu_items(
            sub_idx,
            state,
            &context.resolver,
            &context.config.settings,
        );

        let submenu_width = 37;
        let submenu_height = (sub_items.len() + 2) as u16;
        let mut submenu_x = dropdown_x + dropdown_width;
        if submenu_x + submenu_width > size.width {
            submenu_x = dropdown_x.saturating_sub(submenu_width);
        }
        let submenu_y = 1 + (active_item_idx as u16) + 1;

        let submenu_rect =
            Rect::new(submenu_x, submenu_y, submenu_width, submenu_height).intersection(size);
        f.render_widget(Clear, submenu_rect);

        let mut sub_lines = Vec::new();
        for (i, item) in sub_items.iter().enumerate() {
            let is_cursor = i == sub_item_idx;
            let base_style = if is_cursor {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };

            if item.is_separator {
                sub_lines.push(Line::from(Span::styled(
                    " ───────────────────────────────── ",
                    base_style,
                )));
                continue;
            }

            let hotkey_style = if is_cursor {
                base_style.fg(ratatui::style::Color::Yellow)
            } else {
                base_style
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            };

            let mut line_spans = Vec::new();
            let active_char = if item.active { "•" } else { " " };
            line_spans.push(Span::styled(format!(" {} ", active_char), base_style));

            let parsed = crate::ui::hotkey::parse_hotkey(&item.label);
            let label_spans =
                crate::ui::hotkey::render_hotkey_spans(&item.label, base_style, hotkey_style);
            line_spans.extend(label_spans);

            let label_len = parsed.clean_text.chars().count();
            let padding = 25usize.saturating_sub(label_len);
            line_spans.push(Span::styled(" ".repeat(padding), base_style));

            line_spans.push(Span::styled(format!("{:<7}", item.shortcut), base_style));

            sub_lines.push(Line::from(line_spans));
        }

        let sub_paragraph = Paragraph::new(sub_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .style(Style::default().bg(parse_color(&theme.popup_bg))),
        );
        f.render_widget(sub_paragraph, submenu_rect);
    }
}
