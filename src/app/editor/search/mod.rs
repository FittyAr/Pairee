//! Search and replace in the built-in editor.
//!
//! Every search, plain text or regular expression, is compiled to one
//! [`Matcher`] (the `regex` crate; plain text is escaped). Matches never
//! span lines and always start and end on grapheme boundaries, so a match
//! can be selected and replaced like any other text.

mod editor_ops;
mod find;

pub use editor_ops::{FindOutcome, LastSearch};
pub use find::{find, replace_all_edits};

use regex::{Regex, RegexBuilder};
use std::ops::Range;
use unicode_segmentation::GraphemeCursor;

/// The options of the find / replace dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_words: bool,
    /// The pattern is a regular expression and the replacement may use
    /// `$1` / `${name}` for its groups.
    pub regex: bool,
    pub backward: bool,
    /// Continue from the other end of the file when nothing is left.
    pub wrap: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_words: false,
            regex: false,
            backward: false,
            wrap: true,
        }
    }
}

/// A compiled search pattern.
#[derive(Debug, Clone)]
pub struct Matcher {
    regex: Regex,
    whole_words: bool,
    /// Replacements are inserted as typed (no `$1` expansion).
    literal: bool,
}

/// `true` for characters that make up words (whole-word matching).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `true` when byte `at` (a char boundary) starts or ends a grapheme;
/// looks only at the characters around it.
fn is_grapheme_boundary(line: &str, at: usize) -> bool {
    GraphemeCursor::new(at, line.len(), true)
        .is_boundary(line, 0)
        .unwrap_or(false)
}

impl Matcher {
    /// Compiles `pattern`; an invalid regular expression gives its error.
    /// An empty pattern matches nothing (`Ok(None)`).
    pub fn new(pattern: &str, options: SearchOptions) -> Result<Option<Self>, regex::Error> {
        if pattern.is_empty() {
            return Ok(None);
        }
        let source = if options.regex {
            pattern.to_string()
        } else {
            regex::escape(pattern)
        };
        let regex = RegexBuilder::new(&source)
            .case_insensitive(!options.case_sensitive)
            .build()?;
        Ok(Some(Self {
            regex,
            whole_words: options.whole_words,
            literal: !options.regex,
        }))
    }

    /// Byte ranges of the matches in `line`, left to right.
    pub fn matches_in<'a>(&'a self, line: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
        self.regex
            .find_iter(line)
            .map(|m| m.range())
            .filter(move |r| self.accepts(line, r))
    }

    fn accepts(&self, line: &str, r: &Range<usize>) -> bool {
        if !is_grapheme_boundary(line, r.start) || !is_grapheme_boundary(line, r.end) {
            return false;
        }
        !self.whole_words
            || (!line[..r.start]
                .chars()
                .next_back()
                .is_some_and(is_word_char)
                && !line[r.end..].chars().next().is_some_and(is_word_char))
    }

    /// `true` when `range` of `line` is exactly one of its matches.
    pub fn is_match_at(&self, line: &str, range: &Range<usize>) -> bool {
        self.matches_in(line)
            .take_while(|r| r.start <= range.start)
            .any(|r| r == *range)
    }

    /// The text that replaces match `range` of `line`: `replacement` with
    /// its `$1` / `${name}` groups expanded in regex mode.
    pub fn replacement(&self, line: &str, range: &Range<usize>, replacement: &str) -> String {
        if self.literal {
            return replacement.to_string();
        }
        let mut out = String::new();
        if let Some(caps) = self.regex.captures_at(line, range.start) {
            caps.expand(replacement, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(pattern: &str, f: impl FnOnce(&mut SearchOptions)) -> Matcher {
        let mut options = SearchOptions::default();
        f(&mut options);
        Matcher::new(pattern, options).unwrap().unwrap()
    }

    fn all(m: &Matcher, line: &str) -> Vec<Range<usize>> {
        m.matches_in(line).collect()
    }

    #[test]
    fn plain_text_is_escaped_and_case_folds() {
        let m = matcher("a.b", |_| {});
        assert_eq!(all(&m, "A.B axb a.b"), vec![0..3, 8..11]);
        let m = matcher("Ñandú", |o| o.case_sensitive = true);
        assert_eq!(all(&m, "ñandú Ñandú"), vec![8..15]);
        let m = matcher("ÑANDÚ", |_| {});
        assert_eq!(all(&m, "ñandú"), vec![0..7]);
        assert!(
            Matcher::new("", SearchOptions::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn whole_words_use_unicode_word_characters() {
        let m = matcher("año", |o| o.whole_words = true);
        assert_eq!(all(&m, "año años _año (año)"), vec![0..4, 18..22]);
        let m = matcher("-x", |o| o.whole_words = true);
        assert_eq!(all(&m, "a-x -x"), vec![4..6], "'a' touches the first one");
    }

    #[test]
    fn matches_never_split_graphemes() {
        // "e" + combining acute is one grapheme: a bare "e" is not in it.
        let m = matcher("e", |_| {});
        assert_eq!(all(&m, "e\u{301} e"), vec![4..5]);
        let m = matcher("👍", |_| {});
        assert!(all(&m, "👍🏽").is_empty(), "skin tone modifier is part of it");
    }

    #[test]
    fn regex_groups_expand_in_replacements() {
        let m = matcher(r"(\w+)@(\w+)", |o| o.regex = true);
        let line = "mail ana@host now";
        let r = m.matches_in(line).next().unwrap();
        assert_eq!(m.replacement(line, &r, "$2 at ${1}!"), "host at ana!");
        let m = matcher("a$1", |_| {});
        let r = m.matches_in("xa$1").next().unwrap();
        assert_eq!(
            m.replacement("xa$1", &r, "$1"),
            "$1",
            "plain mode is literal"
        );
        assert!(
            Matcher::new(
                "(",
                SearchOptions {
                    regex: true,
                    ..SearchOptions::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn exact_match_check() {
        let m = matcher("ab", |_| {});
        assert!(m.is_match_at("xxab ab", &(2..4)));
        assert!(!m.is_match_at("xxab ab", &(1..3)));
    }
}
