//! Word wrapping by display width (help, release notes, About box, plugin
//! manager details). The only wrapping helper in the UI: [`wrap_lines`] keeps
//! span styles, [`wrap_text`] is the plain-text form.

use crate::ui::text_width::{display_width, split_at_column};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Wraps `lines` at `width` display columns, breaking at whitespace and
/// splitting words longer than a line. Whitespace at a break is dropped.
/// A `width` of 0 is treated as 1 so the loop always makes progress.
pub fn wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut wrapped = Vec::new();
    for line in lines {
        let total: usize = line.spans.iter().map(|s| display_width(&s.content)).sum();
        if total <= width {
            wrapped.push(line);
            continue;
        }
        let mut out = LineBuilder::new(width);
        for span in line.spans {
            for (token, is_space) in tokens(&span.content) {
                out.push(token, is_space, span.style);
            }
        }
        wrapped.extend(out.finish());
    }
    wrapped
}

/// Plain-text form of [`wrap_lines`]: each `\n`-separated line of `text` is
/// wrapped at `width` display columns.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let lines = text.lines().map(|l| Line::raw(l.to_string())).collect();
    wrap_lines(lines, width)
        .into_iter()
        .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect()
}

/// Splits `text` into alternating word / whitespace tokens.
fn tokens(text: &str) -> Vec<(&str, bool)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space = None;
    for (i, c) in text.char_indices() {
        let space = c.is_whitespace();
        if in_space.is_some_and(|s| s != space) {
            out.push((&text[start..i], !space));
            start = i;
        }
        in_space = Some(space);
    }
    if start < text.len() {
        out.push((&text[start..], in_space.unwrap_or(false)));
    }
    out
}

/// Accumulates styled tokens into lines of at most `width` columns.
struct LineBuilder {
    width: usize,
    done: Vec<Line<'static>>,
    spans: Vec<Span<'static>>,
    used: usize,
}

impl LineBuilder {
    fn new(width: usize) -> Self {
        Self {
            width,
            done: Vec::new(),
            spans: Vec::new(),
            used: 0,
        }
    }

    fn break_line(&mut self) {
        self.done.push(Line::from(std::mem::take(&mut self.spans)));
        self.used = 0;
    }

    fn push(&mut self, token: &str, is_space: bool, style: Style) {
        let w = display_width(token);
        if is_space {
            if self.used > 0 && self.used + w <= self.width {
                self.spans.push(Span::styled(token.to_string(), style));
                self.used += w;
            }
            return;
        }
        if self.used + w > self.width && self.used > 0 {
            self.break_line();
        }
        let mut rest = token;
        while display_width(rest) > self.width - self.used {
            let (head, tail) = split_at_column(rest, self.width - self.used);
            let (head, tail) = if head.is_empty() {
                // A grapheme wider than the line: emit it alone.
                let first = tail.chars().next().map_or(0, char::len_utf8);
                tail.split_at(first)
            } else {
                (head, tail)
            };
            self.spans.push(Span::styled(head.to_string(), style));
            self.break_line();
            rest = tail;
        }
        if !rest.is_empty() {
            self.spans.push(Span::styled(rest.to_string(), style));
            self.used += display_width(rest);
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if !self.spans.is_empty() {
            self.done.push(Line::from(self.spans));
        }
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_whitespace_and_drops_break_space() {
        assert_eq!(wrap_text("aaa bbb ccc", 7), vec!["aaa bbb", "ccc"]);
    }

    #[test]
    fn splits_long_words() {
        assert_eq!(wrap_text("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }

    #[test]
    fn zero_width_terminates() {
        let lines = wrap_text("abc def", 0);
        assert_eq!(lines, vec!["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn utf8_and_wide_chars_fit_in_columns() {
        let text = "Realiza auditorías de verificación de cumplimiento 日本語の文章";
        for line in wrap_text(text, 15) {
            assert!(display_width(&line) <= 15, "{line}");
        }
        assert_eq!(wrap_text("日本語", 1), vec!["日", "本", "語"]);
    }

    #[test]
    fn keeps_empty_lines_and_styles() {
        assert_eq!(wrap_text("a\n\nb", 5), vec!["a", "", "b"]);
        let style = Style::default().fg(ratatui::style::Color::Red);
        let lines = wrap_lines(vec![Line::from(Span::styled("xx yy", style))], 2);
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|l| l.spans.iter().all(|s| s.style == style))
        );
    }
}
