//! Sorting modes and user menu dialog rendering.

use super::super::{centered_rect_fixed, centered_rect_in};
use crate::app::state::{ActivePanel, SortField};
use crate::config::localization::t;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

pub fn render_sort_modes_dialog(
    f: &mut Frame,
    theme: &ResolvedTheme,
    size: Rect,
    current: &SortField,
    reverse: bool,
    cursor_idx: usize,
) {
    let area = centered_rect_in(45, 35, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .title(t("popup_sort_modes"))
        .style(Style::default().bg(theme.popup_bg));

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
            theme.selection().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.popup_fg)
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
        theme.selection().add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.popup_fg)
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(line_str, style)));

    let inner = block.inner(area);
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
    // A click picks the row and applies it (Space), keeping the dialog open.
    let rows = (0..fields.len()).map(|i| (i, inner.y + i as u16));
    let reverse_row = std::iter::once((fields.len(), inner.y + fields.len() as u16 + 1));
    for (i, y) in rows.chain(reverse_row) {
        if y < inner.bottom() {
            let mut keys = crate::ui::popup::kit::mouse::presses(
                i as i64 - cursor_idx as i64,
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyCode::Up,
            );
            keys.push(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(' '),
            ));
            crate::ui::popup::kit::mouse::keys(Rect::new(inner.x, y, inner.width, 1), keys);
        }
    }
}

pub fn render_user_menu_dialog(
    f: &mut Frame,
    theme: &ResolvedTheme,
    (left_rect, right_rect): (Rect, Rect),
    active_panel: ActivePanel,
    (items, cursor_idx): (&[crate::app::state::popup::UserMenuItem], usize),
) {
    let panel_rect = match active_panel {
        ActivePanel::Left => left_rect,
        ActivePanel::Right => right_rect,
    };
    // Borders and the header row take three lines; the panel caps the height.
    let height = u16::try_from(items.len() + 3).unwrap_or(u16::MAX);
    let area = centered_rect_fixed(panel_rect.width * 4 / 5, height, panel_rect);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .title(t("popup_user_menu"))
        .style(Style::default().bg(theme.popup_bg));

    let mut menu_rows = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let style = if is_cursor {
            theme.selection().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.popup_fg)
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
    // Rows start under the borders and the header row.
    let rows = area.inner(ratatui::layout::Margin::new(1, 1));
    let rows = Rect {
        y: rows.y + 1,
        height: rows.height.saturating_sub(1),
        ..rows
    };
    crate::ui::popup::kit::mouse::list_rows(rows, 0, items.len(), cursor_idx);
}
