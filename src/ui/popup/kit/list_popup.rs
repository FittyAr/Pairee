//! Bordered list popup with a highlighted cursor row, optional header lines,
//! a hint line and an optional scrollbar (history lists, tree view, folder
//! comparison, task list...).

use super::{fg, frame_in, popup_fg, selection};
use crate::config::theme::Theme;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
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
    /// Dim help line under the list (after a blank line).
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
        let hint_rows = if self.hint.is_some() { 2 } else { 0 }; // blank line + hint
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
            lines.push(Line::from(Span::styled(hint, fg(Color::DarkGray))));
        }
        f.render_widget(Paragraph::new(lines).style(popup_fg(theme)), inner);

        if let Some((hits, id)) = self.scrollbar {
            scrollbar::render_vertical_right(
                f,
                inner,
                total,
                height,
                start,
                theme,
                ScrollbarSurface::Popup,
                hits,
                id,
            );
        }
    }
}
