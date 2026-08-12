//! Render the command palette popup.

use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, area: Rect) -> bool {
    let PopupType::CommandPalette {
        query,
        cursor_idx,
        items,
    } = popup
    else {
        return false;
    };

    let width = area.width.min(64);
    let height = area.height.min(18);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 3;
    let rect = Rect::new(x, y, width, height);

    f.render_widget(Clear, rect);
    let block = Block::default()
        .title(" Command Palette (Ctrl+Shift+P) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .style(
            Style::default()
                .fg(parse_color(&theme.popup_fg))
                .bg(parse_color(&theme.popup_bg)),
        );
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(query.as_str()),
    ]))
    .style(
        Style::default()
            .fg(parse_color(&theme.popup_fg))
            .bg(parse_color(&theme.popup_bg)),
    );
    f.render_widget(prompt, chunks[0]);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let style = if i == *cursor_idx {
                Style::default()
                    .fg(parse_color(&theme.selection_fg))
                    .bg(parse_color(&theme.selection_bg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(parse_color(&theme.popup_fg))
                    .bg(parse_color(&theme.popup_bg))
            };
            ListItem::new(label.as_str()).style(style)
        })
        .collect();

    f.render_widget(List::new(list_items), chunks[1]);
    true
}
