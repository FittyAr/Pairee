//! Syntax highlighting of the text shown by the built-in editor and viewer.
//!
//! Pairee's own highlighter, in the style of the micro editor: every
//! language is a TOML file of regular expressions (`definition.rs`), the
//! built-in ones embedded from the `syntax/` folder (`builtin.rs`) and the
//! user's read from the configuration directory (`registry.rs`). Rules
//! color matches and regions (strings, comments) span lines; the state
//! carried between lines is the list of open regions (`highlight.rs`).
//!
//! This module does not depend on the terminal backend: it yields byte
//! ranges of each line tagged with a [`SyntaxKind`], and the UI paints
//! every kind with a color of the active theme.
//!
//! Only the lines on screen are highlighted. [`SyntaxCache`] remembers the
//! line state every few lines, so scrolling and typing re-parse a few
//! dozen lines rather than the whole file (see `cache.rs`).

mod builtin;
mod cache;
mod definition;
mod grammar;
mod highlight;
mod language;
mod painter;
mod registry;
mod template;
#[cfg(test)]
mod tests;

pub use cache::SyntaxCache;
pub use registry::load_user_definitions;

use std::ops::Range;

/// The token classes the theme gives colors to (named as the theme's
/// `[syntax]` keys in definition files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Highlights `text` line by line with the language defined by
/// `definition` (a user file), which may embed itself and the built-in
/// languages. Errors are the ones a user would see for the file.
#[cfg(any(test, feature = "fuzzing"))]
pub fn highlight_with_definition(definition: &str, text: &str) -> Result<Vec<LineTokens>, String> {
    let definition = definition::Definition::parse(definition)?;
    let language = std::sync::Arc::new(language::Language::new(definition)?);
    language.check()?;
    let registry = registry::Registry::new(vec![std::sync::Arc::clone(&language)]);
    let root = registry.root_of(&language);
    let grammars = highlight::Grammars {
        root: root.grammar().ok_or("no grammar")?,
        registry: &registry,
    };
    let mut state = highlight::LineState::default();
    Ok(text.lines().map(|l| state.advance(grammars, l)).collect())
}

/// When highlighting applies, from the user's settings (off by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// large or no language knows it.
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
