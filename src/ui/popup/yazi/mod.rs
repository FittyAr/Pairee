pub mod sort;
pub mod view;

pub use sort::build_sort_rows;
pub use view::build_view_rows;

use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Table},
};

pub fn render_yazi_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (title, rows) = match popup {
        PopupType::YaziSortPopup => (" Sort modes (Yazi style) ", build_sort_rows(theme)),
        PopupType::YaziViewPopup => (" View modes (Yazi style) ", build_view_rows(theme)),
        _ => return false,
    };

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
    true
}
