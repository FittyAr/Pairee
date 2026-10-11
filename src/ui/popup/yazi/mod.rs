pub mod sort;
pub mod view;

use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table},
};

/// `(key, label)` cells of a key popup: rows of four, `None` for an empty cell.
pub type KeyGrid = [[Option<(&'static str, &'static str)>; 4]; 3];

/// Table rows of `grid`: yellow key, `⋄`, label.
pub fn key_rows(theme: &crate::config::theme::Theme, grid: &KeyGrid) -> Vec<Row<'static>> {
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(parse_color(&theme.popup_fg));
    grid.iter()
        .map(|row| {
            Row::new(row.iter().map(|cell| match cell {
                Some((key, label)) => Cell::from(Line::from(vec![
                    Span::styled(*key, key_style),
                    Span::styled("⋄", Style::default().fg(Color::DarkGray)),
                    Span::styled(*label, label_style),
                ])),
                None => Cell::from(""),
            }))
        })
        .collect()
}

pub fn render_yazi_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (title, grid) = match popup {
        PopupType::YaziSortPopup => (" Sort modes (Yazi style) ", &sort::SORT_KEYS),
        PopupType::YaziViewPopup => (" View modes (Yazi style) ", &view::VIEW_KEYS),
        _ => return false,
    };
    let rows = key_rows(theme, grid);

    let panel_height = 4;
    let area = Rect::new(
        0,
        size.height.saturating_sub(panel_height),
        size.width,
        panel_height,
    );
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .block(block);

    f.render_widget(table, area);
    register_keys(area, grid);
    true
}

/// The strip counts as inside the popup; a click on a cell presses its key.
fn register_keys(area: Rect, grid: &KeyGrid) {
    use crate::ui::hit::{HitTarget, register};
    register(area, HitTarget::PopupBody);
    let rows = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    let columns = ratatui::layout::Layout::horizontal([Constraint::Percentage(25); 4])
        .spacing(1)
        .split(rows);
    for (y, row) in (rows.y..rows.bottom()).zip(grid) {
        for (column, cell) in columns.iter().zip(row) {
            if let Some(c) = cell.and_then(|(key, _)| key.trim().chars().next()) {
                crate::ui::popup::kit::mouse::keys(
                    Rect::new(column.x, y, column.width, 1),
                    crate::ui::popup::kit::mouse::key(crossterm::event::KeyCode::Char(c)),
                );
            }
        }
    }
}
