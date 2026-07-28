use ratatui::style::Style;
use ratatui::text::Span;

#[derive(Debug, Clone)]
pub struct HotkeyString {
    pub clean_text: String,
    pub hotkey: Option<char>,
}

/// Parses a string with an '&' prefix to indicate a hotkey character.
/// Returns the text without the '&' and the extracted hotkey character (lowercase).
///
/// # Examples
///
/// ```no_run
/// use crate::ui::hotkey::parse_hotkey;
///
/// let parsed = parse_hotkey("&File");
/// assert_eq!(parsed.clean_text, "File");
/// assert_eq!(parsed.hotkey, Some('f'));
///
/// // No ampersand → no hotkey.
/// let plain = parse_hotkey("Help");
/// assert_eq!(plain.hotkey, None);
/// ```
pub fn parse_hotkey(text: &str) -> HotkeyString {
    let mut clean_text = String::new();
    let mut hotkey = None;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' && chars.peek().is_some() {
            let next_c = chars.next().unwrap();
            if hotkey.is_none() {
                hotkey = Some(next_c.to_ascii_lowercase());
            }
            clean_text.push(next_c);
        } else {
            clean_text.push(c);
        }
    }

    HotkeyString { clean_text, hotkey }
}

/// Renders a string with an '&' prefix into a vector of Spans,
/// applying `hotkey_style` to the character immediately following the '&'.
pub fn render_hotkey_spans(
    text: &str,
    base_style: Style,
    hotkey_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' && chars.peek().is_some() {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
            }
            let next_c = chars.next().unwrap();
            spans.push(Span::styled(next_c.to_string(), hotkey_style));
        } else {
            current_text.push(c);
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, base_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_hotkey ────────────────────────────────────────────────────────

    #[test]
    fn parse_hotkey_no_ampersand_returns_text_and_no_hotkey() {
        let r = parse_hotkey("Plain text");
        assert_eq!(r.clean_text, "Plain text");
        assert_eq!(r.hotkey, None);
    }

    #[test]
    fn parse_hotkey_single_ampersand_extracts_lowercase_letter() {
        let r = parse_hotkey("&File");
        assert_eq!(r.clean_text, "File");
        assert_eq!(r.hotkey, Some('f'));
    }

    #[test]
    fn parse_hotkey_lowercases_hotkey() {
        // The convention is that '&F' and '&f' refer to the same
        // hotkey — both must return the lowercase form in the
        // `hotkey` field. The `clean_text` field preserves the
        // original case so the rendered label is unchanged.
        let upper = parse_hotkey("&FILE");
        let lower = parse_hotkey("&file");
        assert_eq!(upper.hotkey, Some('f'));
        assert_eq!(lower.hotkey, Some('f'));
        // clean_text preserves the original case.
        assert_eq!(upper.clean_text, "FILE");
        assert_eq!(lower.clean_text, "file");
    }

    #[test]
    fn parse_hotkey_ampersand_in_middle() {
        let r = parse_hotkey("View &details");
        assert_eq!(r.clean_text, "View details");
        assert_eq!(r.hotkey, Some('d'));
    }

    #[test]
    fn parse_hotkey_second_ampersand_drops_its_ampersand() {
        // Documents the (intentional) current behaviour: after the
        // first `&` is consumed as a hotkey, any further `&` is still
        // recognised as a marker — meaning the ampersand itself is
        // stripped from the output, the following char is appended
        // literally, and the hotkey is NOT updated. This is a known
        // quirk (likely a bug) and the test pins the behaviour so a
        // future fix shows up as a deliberate test change.
        let r = parse_hotkey("&A&bout");
        assert_eq!(r.hotkey, Some('a'));
        // The ampersand after 'A' is consumed; clean_text is "About".
        assert_eq!(r.clean_text, "About");
    }

    #[test]
    fn parse_hotkey_ampersand_only_is_a_literal() {
        // A trailing `&` with nothing after is treated as a literal
        // (not a hotkey), because the parser checks for the
        // `chars.peek().is_some()` guard.
        let r = parse_hotkey("Rock&");
        assert_eq!(r.hotkey, None);
        assert_eq!(r.clean_text, "Rock&");
    }

    #[test]
    fn parse_hotkey_empty_string() {
        let r = parse_hotkey("");
        assert_eq!(r.clean_text, "");
        assert_eq!(r.hotkey, None);
    }

    #[test]
    fn parse_hotkey_only_ampersand() {
        let r = parse_hotkey("&");
        assert_eq!(r.hotkey, None);
        assert_eq!(r.clean_text, "&");
    }

    #[test]
    fn parse_hotkey_handles_unicode_ampersand_followed_by_multibyte() {
        // The hotkey is the first char after `&`, regardless of
        // byte width. The clean_text must include the full unicode
        // codepoint.
        let r = parse_hotkey("&ñoño");
        assert_eq!(r.clean_text, "ñoño");
        assert_eq!(r.hotkey, Some('ñ'));
    }

    // ── render_hotkey_spans ─────────────────────────────────────────────────

    #[test]
    fn render_hotkey_spans_no_ampersand_returns_single_base_span() {
        let spans = render_hotkey_spans("File", Style::default(), Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "File");
    }

    #[test]
    fn render_hotkey_spans_with_ampersand_splits_correctly() {
        // "&File" must produce two spans: "F" in hotkey style and
        // "ile" in base style, in that order.
        let base = Style::default();
        let hot = Style::default();
        let spans = render_hotkey_spans("&File", base, hot);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "F");
        assert_eq!(spans[1].content, "ile");
    }

    #[test]
    fn render_hotkey_spans_text_before_and_after() {
        // "View &details" → "View " (base) + "d" (hot) + "etails" (base)
        let spans = render_hotkey_spans("View &details", Style::default(), Style::default());
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "View ");
        assert_eq!(spans[1].content, "d");
        assert_eq!(spans[2].content, "etails");
    }

    #[test]
    fn render_hotkey_spans_empty_string_returns_no_spans() {
        let spans = render_hotkey_spans("", Style::default(), Style::default());
        assert!(spans.is_empty());
    }

    #[test]
    fn render_hotkey_spans_only_ampersand_returns_one_literal_span() {
        // "&" alone (no character after) must be a single base-styled
        // span containing the literal ampersand. The hotkey-style
        // span is suppressed because there's nothing to mark.
        let spans = render_hotkey_spans("&", Style::default(), Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "&");
    }

    // ── Property: parse_hotkey is the inverse of `clean_text + '&' + hotkey` ─

    #[test]
    fn prop_parse_hotkey_round_trip() {
        use proptest::prelude::*;
        proptest!(|(label in "[a-zA-Z0-9 ]{1,20}")| {
            // Build a string with a randomly placed `&` (skip if label
            // contains one, to keep the strategy simple).
            prop_assume!(!label.contains('&'));
            let marked = format!("&{}", label);
            let r = parse_hotkey(&marked);
            // clean_text must equal the original label without the
            // leading ampersand.
            prop_assert_eq!(r.clean_text.as_str(), label.as_str());
            // The hotkey is always the first letter of the label,
            // lowercased.
            let first = label.chars().next().unwrap().to_ascii_lowercase();
            prop_assert_eq!(r.hotkey, Some(first));
        });
    }
}
