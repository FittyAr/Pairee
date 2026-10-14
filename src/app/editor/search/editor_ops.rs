//! Find, replace and replace all on an [`EditorState`].
//!
//! A found match is selected (anchor at one end, cursor at the other), so
//! typing replaces it and the next search continues past it. "Replace"
//! replaces the selection only when it is exactly a match of the search;
//! otherwise it first finds one.

use super::{Matcher, SearchOptions, find, replace_all_edits};
use crate::app::editor::EditorState;
use crate::app::editor::history::Pos;
use crate::app::editor::selection::{Region, Selection};
use std::ops::Range;

/// The search the editor remembers (for "search next / previous" and for
/// highlighting its matches).
#[derive(Debug, Clone)]
pub struct LastSearch {
    pub pattern: String,
    pub replacement: Option<String>,
    pub options: SearchOptions,
    pub matcher: Matcher,
}

/// What a find / replace did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindOutcome {
    Found,
    NotFound,
    /// Replacing was refused: the buffer is locked (read-only file).
    Locked,
}

impl EditorState {
    /// The selected stretch of one line, if the selection is exactly that.
    fn selected_span(&self) -> Option<(usize, Range<usize>)> {
        match self.region()? {
            Region::Stream { start, end } if start.y == end.y => Some((start.y, start.x..end.x)),
            _ => None,
        }
    }

    /// The empty match the last find stopped at, if the cursor is still
    /// there with nothing selected.
    fn found_empty_match(&self) -> Option<(usize, Range<usize>)> {
        let at = self
            .empty_match
            .filter(|&at| at == self.cursor() && self.selection.is_none())?;
        Some((at.y, at.x..at.x))
    }

    /// Moves to the next match of `search` (previous one when `backward`)
    /// and selects it, scrolling it into a `height`-row view.
    pub fn find_match(
        &mut self,
        search: &LastSearch,
        backward: bool,
        height: usize,
    ) -> FindOutcome {
        let cursor = self.cursor();
        let (low, high) = match self.region() {
            Some(Region::Stream { start, end }) => (start, end),
            _ => (cursor, cursor),
        };
        let from = if backward { low } else { high };
        let options = search.options;
        let Some(found) = find(&self.lines, &search.matcher, from, backward, options.wrap) else {
            return FindOutcome::NotFound;
        };
        let (start, end) = (
            Pos::new(found.y, found.range.start),
            Pos::new(found.y, found.range.end),
        );
        let (anchor, head) = if backward { (end, start) } else { (start, end) };
        self.selection = (anchor != head).then_some(Selection::Stream { anchor });
        // An empty match cannot be selected: remember it for "Replace".
        self.empty_match = (anchor == head).then_some(head);
        self.set_cursor(head);
        self.scroll_to_cursor(height);
        FindOutcome::Found
    }

    /// Replaces the selected match with the replacement of `search`, then
    /// finds the next match. Without a selected match it only finds one.
    pub fn replace_match(
        &mut self,
        search: &LastSearch,
        backward: bool,
        height: usize,
    ) -> FindOutcome {
        let current = self
            .selected_span()
            .or_else(|| self.found_empty_match())
            .filter(|(y, r)| search.matcher.is_match_at(&self.lines[*y], r));
        if let Some((y, range)) = current {
            let replacement = search.replacement.as_deref().unwrap_or_default();
            let text = search
                .matcher
                .replacement(&self.lines[y], &range, replacement);
            let region = Region::Stream {
                start: Pos::new(y, range.start),
                end: Pos::new(y, range.end),
            };
            if !self.replace_region(region, &text) {
                return FindOutcome::Locked;
            }
            if backward {
                self.set_cursor(Pos::new(y, range.start));
            }
        }
        self.find_match(search, backward, height)
    }

    /// Replaces every match in the file as one undo step; returns how many
    /// were replaced (`None` when the buffer is locked).
    pub fn replace_all(&mut self, search: &LastSearch) -> Option<usize> {
        let replacement = search.replacement.as_deref().unwrap_or_default();
        let edits = replace_all_edits(&self.lines, &search.matcher, replacement);
        let count = edits.len();
        if count == 0 {
            return Some(0);
        }
        let cursor = self.cursor();
        self.history.seal();
        if !self.edit_group(edits, Some(cursor)) {
            return None;
        }
        self.history.seal();
        Some(count)
    }

    /// Repeats the last search in its direction, or the other way when
    /// `reverse` (search next / previous); `None` without a last search.
    pub fn repeat_search(&mut self, reverse: bool, height: usize) -> Option<FindOutcome> {
        let search = self.last_search.clone()?;
        self.show_matches = true;
        Some(self.find_match(&search, search.options.backward != reverse, height))
    }

    /// Centres the cursor line when it is outside a `height`-row view.
    fn scroll_to_cursor(&mut self, height: usize) {
        if self.cursor_y < self.scroll_y || self.cursor_y >= self.scroll_y + height.max(1) {
            self.scroll_y = self.cursor_y.saturating_sub(height / 2);
        }
    }

    /// Matches of `matcher` on line `y` (for highlighting).
    pub fn line_matches(&self, matcher: &Matcher, y: usize) -> Vec<Range<usize>> {
        self.lines
            .get(y)
            .map(|line| matcher.matches_in(line).filter(|r| !r.is_empty()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(pattern: &str, replacement: &str, f: impl FnOnce(&mut SearchOptions)) -> LastSearch {
        let mut options = SearchOptions::default();
        f(&mut options);
        LastSearch {
            pattern: pattern.into(),
            replacement: Some(replacement.into()),
            options,
            matcher: Matcher::new(pattern, options).unwrap().unwrap(),
        }
    }

    fn selected(ed: &EditorState) -> Option<String> {
        ed.selected_text()
    }

    #[test]
    fn find_selects_matches_both_ways() {
        let mut ed = EditorState::from_text("one two\ntwo one two");
        let s = search("two", "", |_| {});
        assert_eq!(ed.find_match(&s, false, 10), FindOutcome::Found);
        assert_eq!(
            (ed.cursor(), selected(&ed).as_deref()),
            (Pos::new(0, 7), Some("two"))
        );
        ed.find_match(&s, false, 10);
        assert_eq!(ed.cursor(), Pos::new(1, 3));
        ed.find_match(&s, true, 10);
        assert_eq!(
            ed.cursor(),
            Pos::new(0, 4),
            "backward: cursor at the match start"
        );
        ed.find_match(&s, true, 10);
        assert_eq!(ed.cursor(), Pos::new(1, 8), "wrapped to the last match");
        let none = search("zzz", "", |_| {});
        assert_eq!(ed.find_match(&none, false, 10), FindOutcome::NotFound);
    }

    #[test]
    fn replace_needs_a_selected_match_then_moves_on() {
        let mut ed = EditorState::from_text("a1 a2 a3");
        let s = search(r"a(\d)", "b$1", |o| o.regex = true);
        ed.replace_match(&s, false, 10);
        assert_eq!(ed.lines[0], "a1 a2 a3", "first press only finds");
        ed.replace_match(&s, false, 10);
        assert_eq!(ed.lines[0], "b1 a2 a3");
        assert_eq!(selected(&ed).as_deref(), Some("a2"));
        ed.replace_match(&s, true, 10);
        assert_eq!(ed.lines[0], "b1 b2 a3");
        assert_eq!(ed.cursor(), Pos::new(0, 6), "backward wraps to a3");
        assert!(ed.undo());
        assert_eq!(ed.lines[0], "b1 a2 a3", "each replacement is one undo step");
    }

    #[test]
    fn replace_handles_empty_matches() {
        let mut ed = EditorState::from_text("a\nb");
        let s = search("^", "// ", |o| o.regex = true);
        ed.set_cursor(Pos::new(1, 1));
        ed.replace_match(&s, false, 10);
        assert_eq!(ed.lines, vec!["a", "b"], "first press only finds");
        ed.replace_match(&s, false, 10);
        ed.replace_match(&s, false, 10);
        assert_eq!(ed.lines, vec!["// a", "// b"]);
    }

    #[test]
    fn replace_all_is_one_undo_step() {
        let mut ed = EditorState::from_text("Foo foo\nFOO bar foo");
        ed.set_cursor(Pos::new(1, 4));
        let s = search("foo", "baz", |o| o.whole_words = true);
        assert_eq!(ed.replace_all(&s), Some(4));
        assert_eq!(ed.lines, vec!["baz baz", "baz bar baz"]);
        assert_eq!(ed.cursor(), Pos::new(1, 4), "cursor stays");
        assert!(ed.undo());
        assert_eq!(ed.lines, vec!["Foo foo", "FOO bar foo"]);
        assert!(!ed.is_dirty());
        assert!(ed.redo());
        assert_eq!(ed.lines[1], "baz bar baz");
    }

    #[test]
    fn replace_all_handles_graphemes_and_locked_buffers() {
        let mut ed = EditorState::from_text("ñandú e\u{301} e 👍🏽");
        let s = search("e", "é", |_| {});
        assert_eq!(ed.replace_all(&s), Some(1));
        assert_eq!(ed.lines[0], "ñandú e\u{301} é 👍🏽");
        let emoji = search("👍🏽", "ok", |_| {});
        ed.locked = true;
        assert_eq!(ed.replace_all(&emoji), None);
        ed.locked = false;
        ed.find_match(&emoji, false, 10);
        assert_eq!(ed.replace_match(&emoji, false, 10), FindOutcome::NotFound);
        assert_eq!(ed.lines[0], "ñandú e\u{301} é ok");
    }
}
