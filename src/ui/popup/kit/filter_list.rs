//! Type-to-filter list popup (command palette): a `> query` line,
//! the matching rows with the cursor highlighted, and an optional hint.

use crate::app::text_input::TextField;
use crate::ui::resolved_theme::ResolvedTheme;
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
    pub fn render(self, f: &mut Frame, area: Rect, theme: &ResolvedTheme) {
        let width = area.width.min(self.max_width);
        let height = area.height.min(18);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 3;
        let rect = crate::ui::popup::place_popup(Rect::new(x, y, width, height), area);

        let normal = theme.popup_fg().bg(theme.popup_bg);
        f.render_widget(Clear, rect);
        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.popup_border))
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

        // The query with its cursor: a click moves it (PopupField below).
        let mut prompt = vec![Span::styled(
            "> ",
            Style::default().add_modifier(Modifier::BOLD),
        )];
        prompt.extend(super::field_spans(self.query, normal, theme.selection(), true));
        let prompt = Line::from(prompt);
        f.render_widget(Paragraph::new(prompt).style(normal), chunks[0]);
        crate::ui::hit::register(
            chunks[0],
            crate::ui::hit::HitTarget::PopupField {
                keys: Vec::new(),
                text_x: chunks[0].x + 2,
            },
        );

        let cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        let highlighted = theme.selection().add_modifier(Modifier::BOLD);
        // Keep the cursor row in view when the list is taller than the box.
        let height = chunks[1].height as usize;
        let start = crate::ui::scrollbar::centered_scroll(cursor, self.rows.len(), height);
        super::mouse::list_rows(chunks[1], start, self.rows.len(), cursor);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn the_query_shows_where_typing_goes() {
        let theme = ResolvedTheme::default();
        let mut query = TextField::new("abc");
        query.set_cursor(1);
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|f| {
                FilterListView {
                    max_width: 40,
                    title: String::new(),
                    query: &query,
                    rows: Vec::new(),
                    cursor: 0,
                    hint: None,
                }
                .render(f, f.area(), &theme)
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        // "> abc" inside the frame: the cursor cell is the "b".
        let (b, c) = (&buf[(4, 1)], &buf[(5, 1)]);
        assert_eq!(b.symbol(), "b");
        assert_eq!(b.style().bg, theme.selection().bg);
        assert_ne!(c.style().bg, theme.selection().bg);
    }
}
