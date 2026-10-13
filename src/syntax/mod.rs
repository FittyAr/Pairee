//! Syntax highlighting of the text shown by the built-in editor and viewer.
//!
//! Parsing uses `syntect` with its embedded default grammars and the pure
//! Rust `fancy-regex` engine. This module does not depend on the terminal
//! backend: it yields byte ranges of each line tagged with a [`SyntaxKind`],
//! and the UI paints every kind with a color of the active theme.
//!
//! Only the lines on screen are highlighted. [`SyntaxCache`] remembers the
//! parser state every few lines, so scrolling and typing re-parse a few
//! dozen lines rather than the whole file (see `cache.rs`).

mod cache;
mod engine;

pub use cache::SyntaxCache;

use std::ops::Range;

/// The token classes the theme gives colors to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Comment,
    String,
    Keyword,
    /// Numbers, `true` / `false` / `null`, character escapes.
    Constant,
    /// Function names and attributes.
    Function,
    /// Type, class and module names.
    TypeName,
    Operator,
    /// Markup: tags, headings, links.
    Tag,
}

impl SyntaxKind {
    /// Every kind, in the order of their theme colors.
    pub const ALL: [Self; 8] = [
        Self::Comment,
        Self::String,
        Self::Keyword,
        Self::Constant,
        Self::Function,
        Self::TypeName,
        Self::Operator,
        Self::Tag,
    ];
}

/// One highlighted stretch of a line (byte offsets into the line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub range: Range<usize>,
    pub kind: SyntaxKind,
}

/// The tokens of one line, in order and without overlaps. Text outside
/// every token is plain.
pub type LineTokens = Vec<Token>;

/// When highlighting applies, from the user's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxLimits {
    pub enabled: bool,
    /// Larger files open without highlighting.
    pub max_bytes: u64,
}

impl SyntaxLimits {
    pub fn from_settings(settings: &crate::config::settings::Settings) -> Self {
        Self {
            enabled: settings.syntax_highlighting,
            max_bytes: u64::from(settings.syntax_highlighting_max_kb) * 1024,
        }
    }

    /// A highlighter for a file of `size` bytes at `path` whose first line
    /// is `first_line`, or `None` when highlighting is off, the file is too
    /// large or no grammar knows it.
    pub fn cache_for(
        &self,
        path: &std::path::Path,
        first_line: &str,
        size: u64,
    ) -> Option<SyntaxCache> {
        (self.enabled && size <= self.max_bytes)
            .then(|| SyntaxCache::detect(path, first_line))
            .flatten()
    }
}
