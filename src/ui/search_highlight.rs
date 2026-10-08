//! Highlighting of search matches inside a line (viewer and editor).

use ratatui::style::Style;
use ratatui::text::Span;

/// Splits `line` into spans, styling every match of `query` with
/// `highlight_style` and the rest with `normal_style`.
pub fn highlight_line(
    line: &str,
    query: &str,
    case_sensitive: bool,
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(line.to_string(), normal_style)];
    }
    // Fold char by char so both sides keep one entry per `char` of `line`.
    let fold = |c: char| {
        if case_sensitive {
            c
        } else {
            c.to_lowercase().next().unwrap_or(c)
        }
    };
    let chars: Vec<char> = line.chars().collect();
    let folded: Vec<char> = chars.iter().map(|&c| fold(c)).collect();
    let needle: Vec<char> = query.chars().map(fold).collect();
    let matches_at = |i: usize| folded.get(i..i + needle.len()) == Some(needle.as_slice());

    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < chars.len() {
        if matches_at(i) {
            if !plain.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut plain), normal_style));
            }
            let found: String = chars[i..i + needle.len()].iter().collect();
            spans.push(Span::styled(found, highlight_style));
            i += needle.len();
        } else {
            plain.push(chars[i]);
            i += 1;
        }
    }
    if !plain.is_empty() {
        spans.push(Span::styled(plain, normal_style));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn texts(spans: &[Span]) -> Vec<String> {
        spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn marks_matches_case_insensitive() {
        let hl = Style::default().fg(Color::Red);
        let spans = highlight_line("Foo bar FOO", "foo", false, Style::default(), hl);
        assert_eq!(texts(&spans), ["Foo", " bar ", "FOO"]);
        assert_eq!(spans[0].style, hl);
        assert_eq!(spans[2].style, hl);
    }

    #[test]
    fn case_sensitive_and_empty_query() {
        let spans = highlight_line("Foo foo", "foo", true, Style::default(), Style::default());
        assert_eq!(texts(&spans), ["Foo ", "foo"]);
        let spans = highlight_line("abc", "", true, Style::default(), Style::default());
        assert_eq!(texts(&spans), ["abc"]);
    }

    #[test]
    fn expanding_lowercase_does_not_panic() {
        // 'İ' lowercases to two chars; matching must stay aligned.
        let spans = highlight_line("xİyİ", "i", false, Style::default(), Style::default());
        assert_eq!(texts(&spans).concat(), "xİyİ");
    }
}
