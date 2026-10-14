//! Rendering of [`TextField`]: plain spans, or embedded in a localized
//! template.

use crate::app::text_input::{TextField, split_at_cursor};
use crate::ui::text_width::{display_width, skip_columns, split_at_column};
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

/// Spans for a [`TextField`]: the text in `style`, and when `focused` the
/// grapheme under the cursor (or a trailing space) in `cursor`.
fn field_spans(
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

/// A secret field as it is shown: one `*` per character, the cursor where
/// it is in the real text.
pub fn masked(field: &TextField) -> TextField {
    use unicode_segmentation::UnicodeSegmentation;
    let text = field.text();
    let before = text[..field.cursor()].graphemes(true).count();
    let mut shown = TextField::new("*".repeat(text.graphemes(true).count()));
    shown.set_cursor(before);
    shown
}

/// A [`TextField`] as drawn in `width` display columns. When the text and
/// the cursor cell do not fit, the view scrolls sideways only as far as
/// needed to keep the cursor visible (the field remembers where it was,
/// so clicking or moving inside the visible part does not shift the text),
/// and what does not fit is cut at the edges.
///
/// The horizontal offset is worked out here only: painting
/// ([`Self::spans`]) and mapping a click back to a cursor position
/// ([`Self::hit`]) both come from the same view, so they always agree.
pub struct FieldView<'a> {
    field: &'a TextField,
    /// Display columns scrolled out on the left.
    skip: usize,
    width: usize,
}

impl<'a> FieldView<'a> {
    pub fn new(field: &'a TextField, width: usize) -> Self {
        let text = field.text();
        let (before, at, _) = split_at_cursor(text, field.cursor());
        let cursor_start = display_width(before);
        let cursor_end = cursor_start + display_width(at).max(1);
        let mut skip = field.view_start().min(cursor_start);
        if cursor_end > skip.saturating_add(width) {
            skip = cursor_end - width;
        }
        // No blank room on the right while text is scrolled out on the left
        // (but the cell after the text, where the cursor goes at the end).
        let end = display_width(text) + 1;
        if end < skip.saturating_add(width) {
            skip = end.saturating_sub(width);
        }
        // Whole graphemes only: a wide one cut by the edge goes out entirely.
        let skip = display_width(text) - display_width(skip_columns(text, skip));
        field.set_view_start(skip);
        Self { field, skip, width }
    }

    /// The visible part of the field: the text in `style` and, when
    /// `focused`, the cursor cell in `cursor`.
    pub fn spans(&self, style: Style, cursor: Style, focused: bool) -> Vec<Span<'static>> {
        let mut spans = field_spans(self.field, style, cursor, focused);
        let first = skip_columns(&spans[0].content, self.skip).to_string();
        spans[0] = Span::styled(first, spans[0].style);
        // Cut what does not fit after the visible start (and the cursor).
        let mut room = self.width;
        for span in &mut spans {
            let (fits, cut) = split_at_column(&span.content, room);
            if !cut.is_empty() {
                *span = Span::styled(fits.to_string(), span.style);
            }
            room = room.saturating_sub(display_width(&span.content));
        }
        spans
    }

    /// The click map of this view, its text drawn from screen column
    /// `text_x`.
    pub fn hit(&self, text_x: u16) -> FieldHit {
        use unicode_segmentation::UnicodeSegmentation;
        let mut bounds = vec![0u16];
        let mut end = 0usize;
        for g in self.field.text().graphemes(true) {
            end += crate::ui::text_width::grapheme_width(g);
            bounds.push(u16::try_from(end).unwrap_or(u16::MAX));
        }
        FieldHit {
            text_x,
            skip: u16::try_from(self.skip).unwrap_or(u16::MAX),
            bounds,
        }
    }
}

/// Where a drawn text field's graphemes are, to turn a click into a cursor
/// position ([`FieldView::hit`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldHit {
    /// Screen column of the first visible text cell.
    pub text_x: u16,
    /// Display columns scrolled out on the left.
    skip: u16,
    /// Display column where each grapheme starts, then where the text ends.
    bounds: Vec<u16>,
}

impl FieldHit {
    /// Graphemes before the one drawn at screen `column` (all of them past
    /// the end): where a click there puts the cursor. Wide graphemes count
    /// their columns.
    pub fn graphemes_before(&self, column: u16) -> usize {
        let at = column.saturating_sub(self.text_x).saturating_add(self.skip);
        self.bounds
            .windows(2)
            .position(|g| at < g[1])
            .unwrap_or(self.bounds.len() - 1)
    }
}

/// `template` (a localized, possibly multi-line text) with its first `{}`
/// replaced by `field`, drawn with a block cursor when `focused`.
pub fn template_with_field(
    template: &str,
    field: &FieldView,
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
    spans.extend(field.spans(style, cursor, focused));
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
    fn masked_fields_hide_the_text_but_keep_the_cursor() {
        let mut field = TextField::new("pä🙂s");
        field.move_left();
        let shown = masked(&field);
        assert_eq!(shown.text(), "****");
        assert_eq!(shown.cursor(), 3);
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

    fn view_text(field: &TextField, width: usize) -> String {
        let view = FieldView::new(field, width);
        text(&view.spans(Style::default(), Style::default(), true))
    }

    #[test]
    fn long_text_scrolls_to_keep_the_cursor_visible() {
        let mut field = TextField::new("abcdefghij");
        assert_eq!(view_text(&field, 5), "ghij ");
        field.move_home();
        assert_eq!(view_text(&field, 5), "abcde", "cut at the right edge");
        let mut field = TextField::new("日本語テキスト");
        field.move_home();
        let view = FieldView::new(&field, 4);
        let spans = view.spans(Style::default(), Style::default(), true);
        assert_eq!(spans[1].content, "日", "the cursor stays at the start");
        assert_eq!(text(&spans), "日本");
    }

    #[test]
    fn clicks_map_through_the_scroll_and_wide_characters() {
        // "ab日本cd", cursor at the end, 5 columns: "本cd " is visible.
        let field = TextField::new("ab日本cd");
        let view = FieldView::new(&field, 5);
        assert_eq!(view_text(&field, 5), "本cd ");
        let hit = view.hit(10);
        assert_eq!(hit.graphemes_before(10), 3, "本");
        assert_eq!(hit.graphemes_before(11), 3, "second cell of 本");
        assert_eq!(hit.graphemes_before(12), 4, "c");
        assert_eq!(hit.graphemes_before(14), 6, "past the end");
        // Unscrolled, a wide character still takes two columns.
        let hit = FieldView::new(&field, 40).hit(0);
        assert_eq!(hit.graphemes_before(3), 2, "日 spans columns 2-3");
        assert_eq!(hit.graphemes_before(4), 3);
    }

    #[test]
    fn template_keeps_lines_around_the_field() {
        let field = TextField::new("ab");
        let out = template_with_field(
            "\n Name:\n > {}.zip\n hint",
            &FieldView::new(&field, 10),
            Style::default(),
            Style::default(),
            true,
        );
        let lines: Vec<String> = out.lines.iter().map(|l| text(&l.spans)).collect();
        assert_eq!(lines, ["", " Name:", " > ab .zip", " hint"]);
    }
}
