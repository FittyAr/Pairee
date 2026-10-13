//! One line of text painted in layers: syntax colors, then search matches,
//! then the selection (editor and viewer).

use crate::app::editor::columns::cells;
use ratatui::style::Style;
use ratatui::text::Span;
use std::ops::Range;

/// A byte range of the line and its style; later layers paint over
/// earlier ones.
pub type Layer = (Range<usize>, Style);

/// How the line is laid out on screen.
#[derive(Debug, Clone, Copy)]
pub struct LineView {
    pub tab_size: usize,
    /// Display columns scrolled off to the left.
    pub scroll_x: usize,
    /// Paints one extra cell after the text (a selected line break).
    pub trailing: Option<Style>,
}

/// Spans of `line`: tabs expanded, `view.scroll_x` columns skipped (a wide
/// grapheme cut by the edge shows as spaces) and each grapheme styled by
/// the topmost layer covering it, over `base`.
pub fn styled_spans(
    line: &str,
    base: Style,
    layers: &[Layer],
    view: LineView,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut push = |text: &str, style: Style| match spans.last_mut() {
        Some(last) if last.style == style => last.content.to_mut().push_str(text),
        _ => spans.push(Span::styled(text.to_string(), style)),
    };
    let mut end = 0;
    for cell in cells(line, view.tab_size) {
        end = cell.col + cell.width;
        if end <= view.scroll_x {
            continue;
        }
        let style = layers
            .iter()
            .rev()
            .find(|(range, _)| range.contains(&cell.byte))
            .map_or(base, |(_, style)| base.patch(*style));
        if cell.text == "\t" || cell.col < view.scroll_x {
            push(&" ".repeat(end - cell.col.max(view.scroll_x)), style);
        } else {
            push(cell.text, style);
        }
    }
    if let Some(style) = view.trailing.filter(|_| end >= view.scroll_x) {
        push(" ", style);
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn view(scroll_x: usize) -> LineView {
        LineView {
            tab_size: 4,
            scroll_x,
            trailing: None,
        }
    }

    fn texts(spans: &[Span<'_>]) -> Vec<String> {
        spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn upper_layers_win_and_runs_merge() {
        let red = Style::default().fg(Color::Red);
        let sel = Style::default().bg(Color::Blue);
        let layers = [(0..6, red), (2..4, sel)];
        let spans = styled_spans("abcdefg", Style::default(), &layers, view(0));
        assert_eq!(texts(&spans), ["ab", "cd", "ef", "g"]);
        assert_eq!(spans[0].style, red);
        assert_eq!(spans[1].style, sel);
        assert_eq!(spans[3].style, Style::default());
    }

    #[test]
    fn tabs_wide_graphemes_and_scrolling() {
        let spans = styled_spans("a\tb日c", Style::default(), &[], view(0));
        assert_eq!(texts(&spans).concat(), "a   b日c");
        let spans = styled_spans("a\tb日c", Style::default(), &[], view(6));
        assert_eq!(texts(&spans).concat(), " c", "half of 日 is a space");
        let trailing = LineView {
            trailing: Some(Style::default()),
            ..view(9)
        };
        assert!(styled_spans("abc", Style::default(), &[], trailing).is_empty());
    }
}
