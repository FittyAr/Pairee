//! Patterns that use the text captured by a region's start: `${1}` in an
//! `end` (a heredoc word, the `#`s of a Rust raw string, the fence of a
//! Markdown code block) or in an `include` (the language of a code block).
//!
//! An end is rendered with the captured text escaped and compiled when the
//! region opens; [`EndCache`] keeps the last few compiled ends of a region
//! so a file with many heredocs does not recompile the same pattern.

use regex::{Captures, Regex};
use std::sync::{Arc, Mutex, PoisonError};

/// Compiled ends kept per region.
const CACHED_ENDS: usize = 16;
/// Longest captured text used in a pattern (longer captures are cut).
const MAX_CAPTURE_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Group(usize),
}

/// A pattern or name with `${N}` references to the start's capture groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pieces: Vec<Piece>,
}

impl Template {
    /// The template in `text`, or `None` when it references no group.
    pub fn parse(text: &str) -> Option<Self> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut rest = text;
        while let Some(at) = rest.find("${") {
            let after = &rest[at + 2..];
            let digits = after.bytes().take_while(u8::is_ascii_digit).count();
            let group = after[..digits].parse::<usize>().ok();
            match group.filter(|_| after[digits..].starts_with('}')) {
                Some(group) => {
                    literal.push_str(&rest[..at]);
                    if !literal.is_empty() {
                        pieces.push(Piece::Text(std::mem::take(&mut literal)));
                    }
                    pieces.push(Piece::Group(group));
                    rest = &after[digits + 1..];
                }
                None => {
                    literal.push_str(&rest[..at + 2]);
                    rest = after;
                }
            }
        }
        literal.push_str(rest);
        if !literal.is_empty() {
            pieces.push(Piece::Text(literal));
        }
        pieces
            .iter()
            .any(|p| matches!(p, Piece::Group(_)))
            .then_some(Self { pieces })
    }

    /// The highest group referenced.
    pub fn max_group(&self) -> usize {
        self.pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Group(g) => Some(*g),
                Piece::Text(_) => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// The text with each reference replaced by `group(n)` (empty when the
    /// group did not take part).
    fn render_with<'t>(&self, group: impl Fn(usize) -> &'t str, escape: bool) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Group(g) => {
                    let text = group(*g);
                    let text = &text[..text.floor_char_boundary(MAX_CAPTURE_BYTES)];
                    if escape {
                        out.push_str(&regex::escape(text));
                    } else {
                        out.push_str(text);
                    }
                }
            }
        }
        out
    }

    /// A pattern: the captured text is escaped (matched literally).
    pub fn pattern(&self, caps: &Captures) -> String {
        self.render_with(|g| caps.get(g).map_or("", |m| m.as_str()), true)
    }

    /// A name: the captured text as it is, trimmed.
    pub fn name(&self, caps: &Captures) -> String {
        let text = self.render_with(|g| caps.get(g).map_or("", |m| m.as_str()), false);
        text.trim().to_string()
    }

    /// The pattern with every reference replaced by `sample` (to check at
    /// load time that the end compiles).
    pub fn sample(&self, sample: &str) -> String {
        self.render_with(|_| sample, true)
    }
}

/// Least recently used compiled ends of one region.
#[derive(Debug, Default)]
pub struct EndCache {
    /// Most recently used last.
    entries: Mutex<Vec<(String, Arc<Regex>)>>,
}

impl EndCache {
    /// The compiled `pattern`, built with `build` when not cached.
    pub fn get(
        &self,
        pattern: String,
        build: impl FnOnce(&str) -> Option<Regex>,
    ) -> Option<Arc<Regex>> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(at) = entries.iter().position(|(p, _)| *p == pattern) {
            let hit = entries.remove(at);
            let regex = Arc::clone(&hit.1);
            entries.push(hit);
            return Some(regex);
        }
        let regex = Arc::new(build(&pattern)?);
        if entries.len() >= CACHED_ENDS {
            entries.remove(0);
        }
        entries.push((pattern, Arc::clone(&regex)));
        Some(regex)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_references() {
        assert_eq!(Template::parse("^end$"), None);
        assert_eq!(Template::parse("${x} ${"), None, "not references");
        let t = Template::parse(r"^\s*${1}$").unwrap();
        assert_eq!(t.max_group(), 1);
        let caps = Regex::new(r"<<(\w+)").unwrap().captures("<<EOF").unwrap();
        assert_eq!(t.pattern(&caps), r"^\s*EOF$");
        assert_eq!(Template::parse("${2}x${10}").unwrap().max_group(), 10);
    }

    #[test]
    fn escapes_patterns_but_not_names() {
        let t = Template::parse("${1}").unwrap();
        let caps = Regex::new(r"(.+)").unwrap().captures(" a.b* ").unwrap();
        assert_eq!(t.pattern(&caps), r" a\.b\* ");
        assert_eq!(t.name(&caps), "a.b*");
        let caps = Regex::new(r"(x)?y").unwrap().captures("y").unwrap();
        assert_eq!(t.pattern(&caps), "", "a group that took no part");
    }

    #[test]
    fn long_captures_are_cut() {
        let t = Template::parse("${0}").unwrap();
        let long = "é".repeat(MAX_CAPTURE_BYTES);
        let caps = Regex::new(".+").unwrap().captures(&long).unwrap();
        assert!(t.name(&caps).len() <= MAX_CAPTURE_BYTES);
    }

    #[test]
    fn the_cache_is_bounded_and_reuses_entries() {
        let cache = EndCache::default();
        let build = |p: &str| Regex::new(p).ok();
        let first = cache.get("a".into(), build).unwrap();
        let again = cache.get("a".into(), |_| panic!("cached")).unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        for i in 0..CACHED_ENDS * 2 {
            cache.get(format!("x{i}"), build).unwrap();
        }
        assert_eq!(cache.len(), CACHED_ENDS);
        assert!(cache.get("(".into(), build).is_none());
    }
}
