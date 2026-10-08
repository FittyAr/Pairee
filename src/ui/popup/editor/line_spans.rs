//! Styled spans of one visible editor line: selection highlight, otherwise
//! search-match highlight.

use crate::app::editor::columns::byte_to_column;
use crate::app::editor::selection::Region;
use crate::ui::search_highlight::highlight_line;
use crate::ui::text_width::{display_width, expand_tabs, skip_columns, split_at_column};
use ratatui::style::Style;
use ratatui::text::Span;

/// Styles used to paint editor text.
#[derive(Debug, Clone, Copy)]
pub struct LineStyles {
    pub normal: Style,
    pub search: Style,
    pub selected: Style,
}

/// What to paint on one line besides plain text.
#[derive(Debug, Clone, Copy)]
pub struct LineMarks<'a> {
    pub region: Option<Region>,
    pub search: Option<(&'a str, bool)>,
    pub tab_size: usize,
    pub scroll_x: usize,
}

/// Spans of line `y` (`line` is its raw text) scrolled by `marks.scroll_x`.
pub fn line_spans(
    y: usize,
    line: &str,
    marks: &LineMarks<'_>,
    styles: &LineStyles,
) -> Vec<Span<'static>> {
    let tab = marks.tab_size;
    let mut expanded = expand_tabs(line, tab);
    let selected = marks.region.and_then(|r| {
        let (from, to) = r.span_on_line(y, line, tab)?;
        let line_break = r.includes_line_break(y);
        if line_break {
            // Show the selected line break as one highlighted cell.
            expanded.push(' ');
        }
        let end = byte_to_column(line, to, tab) + usize::from(line_break);
        Some((byte_to_column(line, from, tab), end))
    });
    let visible = skip_columns(&expanded, marks.scroll_x);
    let Some((from, to)) = selected else {
        return match marks.search {
            Some((query, case)) => {
                highlight_line(visible, query, case, styles.normal, styles.search)
            }
            None => vec![Span::styled(visible.to_string(), styles.normal)],
        };
    };
    let (before, rest) = split_at_column(visible, from.saturating_sub(marks.scroll_x));
    let mid_width = to
        .saturating_sub(marks.scroll_x)
        .saturating_sub(display_width(before));
    let (mid, after) = split_at_column(rest, mid_width);
    [
        (before, styles.normal),
        (mid, styles.selected),
        (after, styles.normal),
    ]
    .into_iter()
    .filter(|(text, _)| !text.is_empty())
    .map(|(text, style)| Span::styled(text.to_string(), style))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::editor::history::Pos;
    use ratatui::style::Color;

    fn styles() -> LineStyles {
        LineStyles {
            normal: Style::default(),
            search: Style::default().fg(Color::Red),
            selected: Style::default().bg(Color::Blue),
        }
    }

    fn marks(region: Option<Region>, scroll_x: usize) -> LineMarks<'static> {
        LineMarks {
            region,
            search: None,
            tab_size: 4,
            scroll_x,
        }
    }

    fn texts(spans: &[Span<'_>]) -> Vec<String> {
        spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn highlights_selected_columns_with_tabs() {
        let region = Region::Stream {
            start: Pos::new(0, 1),
            end: Pos::new(0, 3),
        };
        let spans = line_spans(0, "a\tb日c", &marks(Some(region), 0), &styles());
        assert_eq!(texts(&spans), vec!["a", "   b", "日c"]);
        assert_eq!(spans[1].style, styles().selected);
    }

    #[test]
    fn selected_line_break_is_one_cell() {
        let region = Region::Stream {
            start: Pos::new(0, 2),
            end: Pos::new(1, 0),
        };
        let spans = line_spans(0, "abc", &marks(Some(region), 1), &styles());
        assert_eq!(texts(&spans), vec!["b", "c "]);
    }

    #[test]
    fn block_region_on_short_line() {
        let region = Region::Block {
            top: 0,
            bottom: 2,
            left: 1,
            right: 3,
        };
        let spans = line_spans(1, "x", &marks(Some(region), 0), &styles());
        assert_eq!(texts(&spans), vec!["x"]);
    }
}
