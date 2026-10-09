//! Type-to-filter list popup (command palette): a `> query` line,
//! the matching rows with the cursor highlighted, and an optional hint.

use super::{popup_fg, selection};
use crate::app::text_input::TextField;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// What a filter list shows.
pub struct FilterListView<'a> {
    /// Maximum popup width (the height is at most 18 rows).
    pub max_width: u16,
    pub title: String,
    pub query: &'a TextField,
    pub rows: Vec<String>,
    pub cursor: usize,
    pub hint: Option<String>,
}

impl FilterListView<'_> {
    /// Draws the popup centered horizontally, in the upper third of `area`.
    pub fn render(self, f: &mut Frame, area: Rect, theme: &Theme) {
        let width = area.width.min(self.max_width);
        let height = area.height.min(18);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 3;
        let rect = Rect::new(x, y, width, height);

        let normal = popup_fg(theme).bg(parse_color(&theme.popup_bg));
        f.render_widget(Clear, rect);
        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .style(normal);
        let inner = block.inner(rect);
        f.render_widget(block, rect);

        let hint_rows = u16::from(self.hint.is_some());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(hint_rows),
            ])
            .split(inner);

        let prompt = Line::from(vec![
            Span::styled("> ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.query.text().to_string()),
        ]);
        f.render_widget(Paragraph::new(prompt).style(normal), chunks[0]);

        let cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        let highlighted = selection(theme).add_modifier(Modifier::BOLD);
        // Keep the cursor row in view when the list is taller than the box.
        let height = chunks[1].height as usize;
        let start = crate::ui::scrollbar::centered_scroll(cursor, self.rows.len(), height);
        let items: Vec<ListItem> = self
            .rows
            .into_iter()
            .enumerate()
            .skip(start)
            .take(height)
            .map(|(i, row)| {
                ListItem::new(row).style(if i == cursor { highlighted } else { normal })
            })
            .collect();
        f.render_widget(List::new(items), chunks[1]);

        if let Some(hint) = self.hint {
            f.render_widget(Paragraph::new(hint).style(normal), chunks[2]);
        }
    }
}
