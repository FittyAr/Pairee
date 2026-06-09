use ratatui::style::Style;
use ratatui::text::Span;

#[derive(Debug, Clone)]
pub struct HotkeyString {
    pub clean_text: String,
    pub hotkey: Option<char>,
}

/// Parses a string with an '&' prefix to indicate a hotkey character.
/// Returns the text without the '&' and the extracted hotkey character (lowercase).
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
