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

/// [`field_spans`] fitted in `width` columns: when the text is longer, its
/// start is scrolled out of view so the cursor (at the right edge) stays
/// visible, as in a shell.
pub fn field_spans_scrolled(
    field: &TextField,
    style: Style,
    cursor: Style,
    focused: bool,
    width: usize,
) -> Vec<Span<'static>> {
    use crate::ui::text_width::{display_width, skip_columns};
    let (before, at, _) = split_at_cursor(field.text(), field.cursor());
    let at_width = display_width(at).max(1);
    let overflow = (display_width(before) + at_width).saturating_sub(width);
    let mut spans = field_spans(field, style, cursor, focused);
    if overflow > 0 {
        let visible = skip_columns(&spans[0].content, overflow).to_string();
        spans[0] = Span::styled(visible, spans[0].style);
    }
    spans
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
    fn long_text_scrolls_to_keep_the_cursor_visible() {
        let field = TextField::new("abcdefghij");
        let spans = field_spans_scrolled(&field, Style::default(), Style::default(), true, 5);
        assert_eq!(text(&spans), "ghij ");
        let mut field = TextField::new("日本語テキスト");
        field.move_home();
        let spans = field_spans_scrolled(&field, Style::default(), Style::default(), true, 4);
        assert_eq!(spans[1].content, "日", "the cursor stays at the start");
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
