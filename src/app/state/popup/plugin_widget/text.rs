//! Styled text of plugin widgets: style, span, line and text. Plain data
//! (colors as the names or `#rrggbb` strings the theme files use); the UI
//! layer turns them into ratatui values.

/// Text modifiers, with the same bits as ratatui's `Modifier`.
pub mod modifier {
    pub const BOLD: u16 = 1;
    pub const DIM: u16 = 1 << 1;
    pub const ITALIC: u16 = 1 << 2;
    pub const UNDERLINED: u16 = 1 << 3;
    pub const SLOW_BLINK: u16 = 1 << 4;
    pub const REVERSED: u16 = 1 << 6;
    pub const HIDDEN: u16 = 1 << 7;
    pub const CROSSED_OUT: u16 = 1 << 8;
}

/// Colors and modifiers; `None` / unset bits leave what is underneath.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    /// Modifiers switched on.
    pub add: u16,
    /// Modifiers switched off.
    pub sub: u16,
}

impl WStyle {
    /// `other` applied on top of `self`.
    pub fn patch(&self, other: &WStyle) -> WStyle {
        WStyle {
            fg: other.fg.clone().or_else(|| self.fg.clone()),
            bg: other.bg.clone().or_else(|| self.bg.clone()),
            add: (self.add & !other.sub) | other.add,
            sub: (self.sub & !other.add) | other.sub,
        }
    }

    /// Switches modifier `bit` on.
    pub fn with(&mut self, bit: u16) {
        self.add |= bit;
        self.sub &= !bit;
    }

    /// Drops every color and modifier (`:reset()`).
    pub fn reset(&mut self) {
        *self = WStyle {
            fg: Some("reset".into()),
            bg: Some("reset".into()),
            add: 0,
            sub: u16::MAX,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WSpan {
    pub content: String,
    pub style: WStyle,
    /// Token class of highlighted code (`pairee.preview_code`): the UI
    /// colors it with the active theme's syntax colors, under `style`.
    pub syntax: Option<crate::syntax::SyntaxKind>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WLine {
    pub spans: Vec<WSpan>,
    pub style: WStyle,
    pub align: Option<WAlign>,
}

impl WLine {
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            spans: vec![WSpan {
                content: text.into(),
                ..WSpan::default()
            }],
            ..Self::default()
        }
    }
}

/// Lines of a text, or a string with ANSI escape sequences that the UI
/// decodes (`ui.Text.parse`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WTextBody {
    Lines(Vec<WLine>),
    Ansi(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WText {
    pub body: WTextBody,
    pub style: WStyle,
    pub align: Option<WAlign>,
}

impl Default for WText {
    fn default() -> Self {
        Self::lines(Vec::new())
    }
}

impl WText {
    pub fn lines(lines: Vec<WLine>) -> Self {
        Self {
            body: WTextBody::Lines(lines),
            style: WStyle::default(),
            align: None,
        }
    }

    /// One plain line per line of `text`.
    pub fn raw(text: &str) -> Self {
        Self::lines(text.lines().map(WLine::raw).collect())
    }

    /// Number of lines (ANSI text counts its line breaks).
    pub fn height(&self) -> usize {
        match &self.body {
            WTextBody::Lines(lines) => lines.len(),
            WTextBody::Ansi(text) => text.lines().count(),
        }
    }

    /// Characters held (for the size budget).
    pub fn bytes(&self) -> usize {
        match &self.body {
            WTextBody::Lines(lines) => lines
                .iter()
                .flat_map(|l| &l.spans)
                .map(|s| s.content.len())
                .sum(),
            WTextBody::Ansi(text) => text.len(),
        }
    }

    /// Spans held (for the size budget).
    pub fn spans(&self) -> usize {
        match &self.body {
            WTextBody::Lines(lines) => lines.iter().map(|l| l.spans.len().max(1)).sum(),
            WTextBody::Ansi(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_layers_colors_and_modifiers() {
        let mut base = WStyle {
            fg: Some("red".into()),
            ..WStyle::default()
        };
        base.with(modifier::BOLD);
        let top = WStyle {
            bg: Some("blue".into()),
            sub: modifier::BOLD,
            ..WStyle::default()
        };
        let both = base.patch(&top);
        assert_eq!(both.fg.as_deref(), Some("red"));
        assert_eq!(both.bg.as_deref(), Some("blue"));
        assert_eq!(both.add & modifier::BOLD, 0);
        assert_ne!(both.sub & modifier::BOLD, 0);
    }

    #[test]
    fn raw_text_counts_lines_and_bytes() {
        let text = WText::raw("ab\ncde");
        assert_eq!((text.height(), text.bytes(), text.spans()), (2, 5, 2));
        let ansi = WText {
            body: WTextBody::Ansi("\x1b[31mx\x1b[0m\ny".into()),
            ..WText::default()
        };
        assert_eq!(ansi.height(), 2);
    }
}
