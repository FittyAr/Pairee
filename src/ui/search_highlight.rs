//! Matches of the viewer's search inside a line, for highlighting.

use std::ops::Range;

/// Byte ranges of `line` matching `query` (plain text), left to right and
/// without overlaps.
pub fn match_ranges(line: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    // Fold char by char so both sides keep one entry per `char` of `line`.
    let fold = |c: char| {
        if case_sensitive {
            c
        } else {
            c.to_lowercase().next().unwrap_or(c)
        }
    };
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let needle: Vec<char> = query.chars().map(fold).collect();
    let byte_at = |i: usize| chars.get(i).map_or(line.len(), |(b, _)| *b);
    let mut ranges = Vec::new();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        let hit = chars[i..i + needle.len()]
            .iter()
            .zip(&needle)
            .all(|((_, c), n)| fold(*c) == *n);
        if hit {
            ranges.push(byte_at(i)..byte_at(i + needle.len()));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_matches_case_insensitive() {
        assert_eq!(match_ranges("Foo bar FOO", "foo", false), [0..3, 8..11]);
    }

    #[test]
    fn case_sensitive_and_empty_query() {
        assert_eq!(match_ranges("Foo foo", "foo", true), vec![4..7]);
        assert!(match_ranges("abc", "", true).is_empty());
    }

    #[test]
    fn expanding_lowercase_does_not_panic() {
        // 'İ' lowercases to two chars; matching must stay aligned.
        assert_eq!(match_ranges("xİyİ", "i", false), [1..3, 4..6]);
        assert_eq!(match_ranges("ñandú", "Ú", false), vec![5..7]);
    }
}
