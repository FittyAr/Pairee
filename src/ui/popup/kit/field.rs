//! Rendering of [`TextField`]: plain spans, or embedded in a localized
//! template.

use crate::app::text_input::{TextField, split_at_cursor};
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

/// Spans for a [`TextField`]: the text in `style`, and when `focused` the
/// grapheme under the cursor (or a trailing space) in `cursor`.
pub fn field_spans(
    field: &TextField,
    style: Style,
    cursor: Style,
    focused: bool,
) -> Vec<Span<'static>> {
    if !focused {
        return vec![Span::styled(field.text().to_string(), style)];
    }
    let (before, at, after) = split_at_cursor(field.text(), field.cursor());
    let at = if at.is_empty() { " " } else { at };
    vec![
        Span::styled(before.to_string(), style),
        Span::styled(at.to_string(), cursor),
        Span::styled(after.to_string(), style),
    ]
}

/// `template` (a localized, possibly multi-line text) with its first `{}`
/// replaced by `field`, drawn with a block cursor when `focused`.
pub fn template_with_field(
    template: &str,
    field: &TextField,
    style: Style,
    cursor: Style,
    focused: bool,
) -> Text<'static> {
    let (before, after) = template.split_once("{}").unwrap_or((template, ""));
    let mut before_lines = before.split('\n');
    let head = before_lines.next_back().unwrap_or_default();
    let mut after_lines = after.split('\n');
    let tail = after_lines.next().unwrap_or_default();

    let mut lines: Vec<Line<'static>> = before_lines.map(|l| plain(l, style)).collect();
    let mut spans = vec![Span::styled(head.to_string(), style)];
    spans.extend(field_spans(field, style, cursor, focused));
    spans.push(Span::styled(tail.to_string(), style));
    lines.push(Line::from(spans));
    lines.extend(after_lines.map(|l| plain(l, style)));
    Text::from(lines)
}

fn plain(line: &str, style: Style) -> Line<'static> {
    Line::from(Span::styled(line.to_string(), style))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn field_spans_mark_the_cursor() {
        let mut field = TextField::new("abc");
        field.move_left();
        let spans = field_spans(&field, Style::default(), Style::default(), true);
        assert_eq!(spans[1].content, "c");
        assert_eq!(text(&spans), "abc");
        field.move_end();
        let spans = field_spans(&field, Style::default(), Style::default(), true);
        assert_eq!(text(&spans), "abc ");
        let spans = field_spans(&field, Style::default(), Style::default(), false);
        assert_eq!(text(&spans), "abc");
    }

    #[test]
    fn template_keeps_lines_around_the_field() {
        let field = TextField::new("ab");
        let out = template_with_field(
            "\n Name:\n > {}.zip\n hint",
            &field,
            Style::default(),
            Style::default(),
            true,
        );
        let lines: Vec<String> = out.lines.iter().map(|l| text(&l.spans)).collect();
        assert_eq!(lines, ["", " Name:", " > ab .zip", " hint"]);
    }
}
