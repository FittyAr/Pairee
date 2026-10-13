//! Finding the next / previous match from a position and building the
//! edits of "replace all".

use super::Matcher;
use crate::app::editor::history::{Pos, TextEdit};
use std::ops::Range;

/// A match found by [`find`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub y: usize,
    pub range: Range<usize>,
    /// The search went past the end (or start) of the file to find it.
    pub wrapped: bool,
}

/// First match at or after `from`. A match starting exactly at `from` only
/// counts when `inclusive` or when it is not empty (so repeating a search
/// that found an empty match moves on).
fn forward(
    lines: &[String],
    m: &Matcher,
    from: Pos,
    inclusive: bool,
) -> Option<(usize, Range<usize>)> {
    lines.iter().enumerate().skip(from.y).find_map(|(y, line)| {
        m.matches_in(line)
            .find(|r| {
                y > from.y
                    || r.start > from.x
                    || (r.start == from.x && (inclusive || r.end > r.start))
            })
            .map(|r| (y, r))
    })
}

/// Last match starting before `from` (at `from` too when `inclusive`).
fn backward(
    lines: &[String],
    m: &Matcher,
    from: Pos,
    inclusive: bool,
) -> Option<(usize, Range<usize>)> {
    let last = from.y.min(lines.len().checked_sub(1)?);
    (0..=last).rev().find_map(|y| {
        m.matches_in(&lines[y])
            .filter(|r| y < from.y || r.start < from.x || (inclusive && r.start == from.x))
            .last()
            .map(|r| (y, r))
    })
}

/// The next match after `from` (forward) or before it (`backward`),
/// continuing from the other end of the file when `wrap` is set.
pub fn find(
    lines: &[String],
    m: &Matcher,
    from: Pos,
    backward_dir: bool,
    wrap: bool,
) -> Option<Found> {
    let found = |(y, range), wrapped| Found { y, range, wrapped };
    let hit = if backward_dir {
        backward(lines, m, from, false)
    } else {
        forward(lines, m, from, false)
    };
    if let Some(hit) = hit {
        return Some(found(hit, false));
    }
    if !wrap {
        return None;
    }
    let restart = if backward_dir {
        let y = lines.len().saturating_sub(1);
        backward(
            lines,
            m,
            Pos::new(y, lines.get(y).map_or(0, String::len)),
            true,
        )
    } else {
        forward(lines, m, Pos::default(), true)
    };
    restart.map(|hit| found(hit, true))
}

/// The edits that replace every match in `lines`, last match first so the
/// positions of the earlier ones stay valid while they are applied.
pub fn replace_all_edits(lines: &[String], m: &Matcher, replacement: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    for (y, line) in lines.iter().enumerate().rev() {
        let matches: Vec<Range<usize>> = m.matches_in(line).collect();
        for r in matches.into_iter().rev() {
            let inserted = m.replacement(line, &r, replacement);
            if inserted != line[r.clone()] {
                edits.push(TextEdit {
                    start: Pos::new(y, r.start),
                    removed: line[r].to_string(),
                    inserted,
                });
            }
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::super::SearchOptions;
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn m(pattern: &str, regex: bool) -> Matcher {
        let options = SearchOptions {
            regex,
            ..SearchOptions::default()
        };
        Matcher::new(pattern, options).unwrap().unwrap()
    }

    fn at(found: Option<Found>) -> Option<(usize, usize, bool)> {
        found.map(|f| (f.y, f.range.start, f.wrapped))
    }

    #[test]
    fn forward_backward_and_wrap() {
        let doc = lines(&["The quick fox", "jumps over the dog", "the end"]);
        let the = m("the", false);
        assert_eq!(
            at(find(&doc, &the, Pos::new(0, 0), false, true)),
            Some((0, 0, false))
        );
        assert_eq!(
            at(find(&doc, &the, Pos::new(0, 3), false, true)),
            Some((1, 11, false))
        );
        assert_eq!(
            at(find(&doc, &the, Pos::new(2, 3), false, true)),
            Some((0, 0, true))
        );
        assert_eq!(at(find(&doc, &the, Pos::new(2, 3), false, false)), None);
        assert_eq!(
            at(find(&doc, &the, Pos::new(1, 11), true, true)),
            Some((0, 0, false))
        );
        assert_eq!(
            at(find(&doc, &the, Pos::new(0, 0), true, true)),
            Some((2, 0, true))
        );
        assert_eq!(at(find(&doc, &the, Pos::new(0, 0), true, false)), None);
    }

    #[test]
    fn adjacent_and_empty_matches_make_progress() {
        let doc = lines(&["aaa"]);
        let a = m("a", false);
        assert_eq!(
            at(find(&doc, &a, Pos::new(0, 1), false, false)),
            Some((0, 1, false))
        );
        let start = m("^", true);
        let doc = lines(&["x", "y"]);
        assert_eq!(
            at(find(&doc, &start, Pos::new(0, 0), false, false)),
            Some((1, 0, false))
        );
    }

    #[test]
    fn replace_all_runs_bottom_up_with_groups() {
        let doc = lines(&["a1 b2", "c3"]);
        let digits = m(r"([a-z])(\d)", true);
        let edits = replace_all_edits(&doc, &digits, "$2$1");
        let starts: Vec<Pos> = edits.iter().map(|e| e.start).collect();
        assert_eq!(starts, vec![Pos::new(1, 0), Pos::new(0, 3), Pos::new(0, 0)]);
        let mut out = doc.clone();
        for e in &edits {
            e.apply(&mut out);
        }
        assert_eq!(out, lines(&["1a 2b", "3c"]));
    }

    #[test]
    fn replace_all_skips_unchanged_matches() {
        let doc = lines(&["ab"]);
        assert!(replace_all_edits(&doc, &m("ab", false), "ab").is_empty());
    }
}
