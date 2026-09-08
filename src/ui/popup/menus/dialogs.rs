//! Sorting modes and user menu dialog rendering.

use super::super::{centered_rect, centered_rect_in};
use crate::app::state::{ActivePanel, SortField};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

pub fn render_sort_modes_dialog(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    current: &SortField,
    reverse: bool,
    cursor_idx: usize,
) {
    let area = centered_rect(45, 35, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(t("popup_sort_modes"))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let fields = [
        SortField::Name,
        SortField::Extension,
        SortField::Size,
        SortField::Date,
        SortField::Unsorted,
    ];

    let mut lines = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let is_selected = field == current;
        let active_marker = if is_selected { "√" } else { " " };
        let cursor_marker = if is_cursor { ">" } else { " " };

        let name = match field {
            SortField::Name => t("col_name"),
            SortField::Extension => t("col_extension"),
            SortField::Size => t("col_size"),
            SortField::Date => t("col_date"),
            SortField::Unsorted => t("col_unsorted"),
        };

        let line_str = format!(" {} [{}] {} ", cursor_marker, active_marker, name);
        let style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        lines.push(Line::from(Span::styled(line_str, style)));
    }

    // Reverse setting row
    let is_reverse_cursor = cursor_idx == fields.len();
    let reverse_marker = if reverse { "√" } else { " " };
    let cursor_marker = if is_reverse_cursor { ">" } else { " " };
    let line_str = format!(
        " {} [{}] {} ",
        cursor_marker,
        reverse_marker,
        t("popup_reverse_order")
    );
    let style = if is_reverse_cursor {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(line_str, style)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

pub fn render_user_menu_dialog(
    f: &mut Frame,
    theme: &Theme,
    left_rect: Rect,
    right_rect: Rect,
    active_panel: ActivePanel,
    cursor_idx: usize,
) {
    let panel_rect = match active_panel {
        ActivePanel::Left => left_rect,
        ActivePanel::Right => right_rect,
    };
    let area = centered_rect_in(80, 55, panel_rect);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(t("popup_user_menu"))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let items = crate::app::input_popup::user_menu::get_user_menu_items();
    let mut menu_rows = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        menu_rows.push(Row::new(vec![
            Cell::from(item.key.as_str()).style(style),
            Cell::from(item.label.as_str()).style(style),
        ]));
    }

    let table = Table::new(
        menu_rows,
        [Constraint::Percentage(20), Constraint::Percentage(80)],
    )
    .block(block)
    .header(
        Row::new(vec![Cell::from(t("col_key")), Cell::from(t("col_command"))])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, area);
}
