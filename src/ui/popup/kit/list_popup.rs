//! Bordered list popup with a highlighted cursor row, optional header lines,
//! a hint line and an optional scrollbar (history lists, tree view, folder
//! comparison, task list...).

use super::{fg, frame_in, popup_fg, selection};
use crate::config::theme::Theme;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// How the visible window follows the cursor.
#[derive(Debug, Clone, Copy)]
pub enum Scroll {
    /// Cursor centered, never scrolling past the end (with a scrollbar).
    Centered,
    /// Cursor kept half a page from the top.
    HalfPage,
}

/// A centered box for a list of `rows` entries up to `width` columns wide
/// (plus borders), at least 5 and at most `max_height` rows tall.
pub fn list_area(screen: Rect, width: usize, rows: usize, max_height: u16) -> Rect {
    let width = (width as u16 + 4).min(screen.width.saturating_sub(4));
    let height = (rows as u16 + 2)
        .clamp(5, max_height)
        .min(screen.height.saturating_sub(4));
    crate::ui::popup::centered_rect_fixed(width, height, screen)
}

/// `" >  item "` for the cursor row, `"    item "` otherwise.
pub fn marked(item: &str, is_cursor: bool) -> String {
    format!(" {}  {} ", if is_cursor { ">" } else { " " }, item)
}

pub struct ListPopup<'a> {
    pub area: Rect,
    pub title: String,
    pub border: Color,
    /// Shown instead of the list when there are no rows.
    pub empty: Option<String>,
    /// Lines above the rows (column titles, filter line...).
    pub header: Vec<Line<'static>>,
    /// Every row with its unfocused style.
    pub rows: Vec<(String, Style)>,
    pub cursor: usize,
    pub scroll: Scroll,
    /// Dim help under the list (after a blank line), one row per line.
    pub hint: Option<String>,
    /// Scrollbar hit-testing state and target, when the list has a scrollbar.
    pub scrollbar: Option<(Option<&'a ScrollbarUiState>, ScrollTargetId)>,
}

impl ListPopup<'_> {
    pub fn render(self, f: &mut Frame, theme: &Theme) {
        let inner = frame_in(f, self.area, self.title, fg(self.border), theme);
        if self.rows.is_empty()
            && let Some(empty) = self.empty
        {
            f.render_widget(Paragraph::new(empty).style(popup_fg(theme)), inner);
            return;
        }
        // A blank line, then the hint (one row per line of it).
        let hint_rows = self
            .hint
            .as_ref()
            .map_or(0, |h| 1 + h.lines().count() as u16);
        let reserved = hint_rows + self.header.len() as u16;
        let height = inner.height.saturating_sub(reserved) as usize;
        let start = match self.scroll {
            Scroll::Centered => scrollbar::centered_scroll(self.cursor, self.rows.len(), height),
            Scroll::HalfPage => self.cursor.saturating_sub(height / 2),
        };
        let total = self.rows.len();
        let selected = selection(theme).add_modifier(Modifier::BOLD);
        let mut lines = self.header;
        lines.extend(
            self.rows
                .into_iter()
                .enumerate()
                .skip(start)
                .take(height)
                .map(|(i, (text, style))| {
                    let style = if i == self.cursor { selected } else { style };
                    Line::from(Span::styled(text, style))
                }),
        );
        if let Some(hint) = self.hint {
            lines.push(Line::from(""));
            lines.extend(
                hint.lines()
                    .map(|l| Line::from(Span::styled(l.to_string(), fg(Color::DarkGray)))),
            );
        }
        f.render_widget(Paragraph::new(lines).style(popup_fg(theme)), inner);

        if let Some((hits, id)) = self.scrollbar {
            scrollbar::render_vertical_right(
                f,
                inner,
                ScrollView {
                    content_len: total,
                    viewport_len: height,
                    offset: start,
                },
                theme,
                ScrollTarget {
                    surface: ScrollbarSurface::Popup,
                    hits,
                    id,
                },
            );
        }
    }
}
