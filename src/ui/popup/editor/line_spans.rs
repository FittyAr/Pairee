//! Styled spans of one visible editor line: syntax colors, search matches
//! over them and the selection on top.

use crate::app::editor::selection::Region;
use crate::syntax::Token;
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::styled_line::{Layer, LineView, styled_spans};
use ratatui::style::Style;
use ratatui::text::Span;
use std::ops::Range;

/// Styles used to paint editor text.
#[derive(Debug, Clone, Copy)]
pub struct LineStyles<'a> {
    pub normal: Style,
    pub search: Style,
    pub selected: Style,
    /// Syntax colors.
    pub theme: &'a ResolvedTheme,
}

/// What to paint on one line besides plain text.
#[derive(Debug, Clone, Copy)]
pub struct LineMarks<'a> {
    pub region: Option<Region>,
    pub matches: &'a [Range<usize>],
    pub tokens: &'a [Token],
    pub tab_size: usize,
    pub scroll_x: usize,
}

/// Spans of line `y` (`line` is its raw text) scrolled by `marks.scroll_x`.
pub fn line_spans(
    y: usize,
    line: &str,
    marks: &LineMarks<'_>,
    styles: &LineStyles<'_>,
) -> Vec<Span<'static>> {
    let tab = marks.tab_size;
    let mut layers: Vec<Layer> = marks
        .tokens
        .iter()
        .map(|t| (t.range.clone(), styles.theme.syntax(t.kind)))
        .collect();
    layers.extend(marks.matches.iter().map(|r| (r.clone(), styles.search)));
    let mut trailing = None;
    if let Some(region) = marks.region
        && let Some((from, to)) = region.span_on_line(y, line, tab)
    {
        layers.push((from..to, styles.selected));
        // Show a selected line break as one highlighted cell.
        trailing = region.includes_line_break(y).then_some(styles.selected);
    }
    let view = LineView {
        tab_size: tab,
        scroll_x: marks.scroll_x,
        trailing,
    };
    styled_spans(line, styles.normal, &layers, view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::editor::history::Pos;
    use crate::syntax::SyntaxKind;
    use ratatui::style::Color;

    fn styles(theme: &ResolvedTheme) -> LineStyles<'_> {
        LineStyles {
            normal: Style::default(),
            search: Style::default().fg(Color::Red),
            selected: Style::default().bg(Color::Blue),
            theme,
        }
    }

    fn marks(region: Option<Region>, scroll_x: usize) -> LineMarks<'static> {
        LineMarks {
            region,
            matches: &[],
            tokens: &[],
            tab_size: 4,
            scroll_x,
        }
    }

    fn texts(spans: &[Span<'_>]) -> Vec<String> {
        spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn highlights_selected_columns_with_tabs() {
        let theme = ResolvedTheme::default();
        let region = Region::Stream {
            start: Pos::new(0, 1),
            end: Pos::new(0, 3),
        };
        let spans = line_spans(0, "a\tb日c", &marks(Some(region), 0), &styles(&theme));
        assert_eq!(texts(&spans), vec!["a", "   b", "日c"]);
        assert_eq!(spans[1].style, styles(&theme).selected);
    }

    #[test]
    fn selected_line_break_is_one_cell() {
        let theme = ResolvedTheme::default();
        let region = Region::Stream {
            start: Pos::new(0, 2),
            end: Pos::new(1, 0),
        };
        let spans = line_spans(0, "abc", &marks(Some(region), 1), &styles(&theme));
        assert_eq!(texts(&spans), vec!["b", "c "]);
    }

    #[test]
    fn block_region_on_short_line() {
        let theme = ResolvedTheme::default();
        let region = Region::Block {
            top: 0,
            bottom: 2,
            left: 1,
            right: 3,
        };
        let spans = line_spans(1, "x", &marks(Some(region), 0), &styles(&theme));
        assert_eq!(texts(&spans), vec!["x"]);
    }

    #[test]
    fn syntax_under_matches_under_selection() {
        let theme = ResolvedTheme::default();
        let tokens = [Token {
            range: 0..6,
            kind: SyntaxKind::Keyword,
        }];
        let matches = [2..4, 9..9];
        let region = Region::Stream {
            start: Pos::new(0, 3),
            end: Pos::new(0, 5),
        };
        let marks = LineMarks {
            matches: &matches,
            tokens: &tokens,
            ..marks(Some(region), 0)
        };
        let st = styles(&theme);
        let spans = line_spans(0, "abcdefg", &marks, &st);
        assert_eq!(texts(&spans), ["ab", "c", "de", "f", "g"]);
        let keyword = theme.syntax(SyntaxKind::Keyword);
        let styles_of: Vec<Style> = spans.iter().map(|s| s.style).collect();
        assert_eq!(
            styles_of,
            [keyword, st.search, st.selected, keyword, st.normal]
        );
    }
}
