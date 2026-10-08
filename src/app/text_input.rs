//! Grapheme-aware single-line text editing primitives.
//!
//! Cursor positions are **byte offsets** into the `String` that are always
//! kept on extended grapheme cluster boundaries. Keeping byte offsets (rather
//! than char/grapheme counts) lets callers keep using `split_off`, slicing and
//! `str::find` results directly, while these helpers guarantee that a cursor
//! never lands inside a multi-byte character such as `ñ`, an emoji, or a
//! base letter + combining mark — which would otherwise panic on
//! `String::insert` / `String::remove` or on slicing.

mod field;

pub use field::{TextField, first_paste_line};
use unicode_segmentation::UnicodeSegmentation;

/// Returns the largest grapheme boundary `<= idx` (clamped to `s.len()`).
pub fn floor_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut last = 0;
    for (start, _) in s.grapheme_indices(true) {
        if start > idx {
            break;
        }
        last = start;
    }
    last
}

/// Returns the grapheme boundary immediately before `idx` (0 at the start).
pub fn prev_boundary(s: &str, idx: usize) -> usize {
    let idx = floor_boundary(s, idx);
    s[..idx]
        .grapheme_indices(true)
        .next_back()
        .map(|(start, _)| start)
        .unwrap_or(0)
}

/// Returns the grapheme boundary immediately after `idx` (`s.len()` at the end).
pub fn next_boundary(s: &str, idx: usize) -> usize {
    let idx = floor_boundary(s, idx);
    s[idx..]
        .graphemes(true)
        .next()
        .map(|g| idx + g.len())
        .unwrap_or(s.len())
}

/// Inserts `c` at `*cursor` and advances the cursor past it.
pub fn insert_char(s: &mut String, cursor: &mut usize, c: char) {
    let at = floor_boundary(s, *cursor);
    s.insert(at, c);
    // A combining mark merges with the previous grapheme, so step to the
    // boundary that follows the inserted bytes rather than counting graphemes.
    *cursor = floor_boundary(s, at + c.len_utf8());
    if *cursor < at + c.len_utf8() {
        *cursor = next_boundary(s, *cursor);
    }
}

/// Removes the grapheme before the cursor. Returns `true` when text changed.
pub fn backspace(s: &mut String, cursor: &mut usize) -> bool {
    let end = floor_boundary(s, *cursor);
    if end == 0 {
        *cursor = 0;
        return false;
    }
    let start = prev_boundary(s, end);
    s.replace_range(start..end, "");
    *cursor = start;
    true
}

/// Removes the grapheme at the cursor. Returns `true` when text changed.
pub fn delete(s: &mut String, cursor: &mut usize) -> bool {
    let start = floor_boundary(s, *cursor);
    *cursor = start;
    if start >= s.len() {
        return false;
    }
    let end = next_boundary(s, start);
    s.replace_range(start..end, "");
    true
}

/// Number of graphemes before the cursor (a 0-based "column" for status bars).
pub fn grapheme_col(s: &str, cursor: usize) -> usize {
    s[..floor_boundary(s, cursor)].graphemes(true).count()
}

/// Splits `s` into (before cursor, grapheme at cursor, after it) for rendering
/// a block cursor. The middle part is empty when the cursor is at the end.
pub fn split_at_cursor(s: &str, cursor: usize) -> (&str, &str, &str) {
    let start = floor_boundary(s, cursor);
    let end = next_boundary(s, start);
    (&s[..start], &s[start..end], &s[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(text: &str) -> (String, usize) {
        let mut s = String::new();
        let mut cur = 0;
        for c in text.chars() {
            insert_char(&mut s, &mut cur, c);
        }
        (s, cur)
    }

    #[test]
    fn typing_non_ascii_does_not_panic() {
        let (s, cur) = type_str("ñandú");
        assert_eq!(s, "ñandú");
        assert_eq!(cur, s.len());
        assert_eq!(grapheme_col(&s, cur), 5);
    }

    #[test]
    fn move_and_insert_in_middle_of_accented_word() {
        let (mut s, mut cur) = type_str("ñandú");
        cur = prev_boundary(&s, cur); // before 'ú'
        cur = prev_boundary(&s, cur); // before 'd'
        insert_char(&mut s, &mut cur, 'X');
        assert_eq!(s, "ñanXdú");
        assert_eq!(grapheme_col(&s, cur), 4);
        cur = 0;
        cur = next_boundary(&s, cur);
        assert_eq!(&s[..cur], "ñ");
    }

    #[test]
    fn backspace_and_delete_remove_whole_graphemes() {
        let (mut s, mut cur) = type_str("ñandú");
        assert!(backspace(&mut s, &mut cur));
        assert_eq!(s, "ñand");
        cur = 0;
        assert!(delete(&mut s, &mut cur));
        assert_eq!(s, "and");
        assert!(!backspace(&mut s, &mut cur));
        cur = s.len();
        assert!(!delete(&mut s, &mut cur));
    }

    #[test]
    fn emoji_are_single_graphemes() {
        let (mut s, mut cur) = type_str("a👍🏽b");
        assert_eq!(s.graphemes(true).count(), 3);
        cur = prev_boundary(&s, cur); // before 'b'
        assert!(backspace(&mut s, &mut cur));
        assert_eq!(s, "ab");
        assert_eq!(cur, 1);

        let mut flag = String::from("🇦🇷");
        let mut c = flag.len();
        assert!(backspace(&mut flag, &mut c));
        assert!(flag.is_empty());
    }

    #[test]
    fn combining_marks_join_previous_grapheme() {
        // "e" + U+0301 COMBINING ACUTE ACCENT
        let (mut s, mut cur) = type_str("ce\u{301}x");
        assert_eq!(s.graphemes(true).count(), 3);
        cur = prev_boundary(&s, cur); // before 'x'
        assert_eq!(grapheme_col(&s, cur), 2);
        assert!(backspace(&mut s, &mut cur));
        assert_eq!(s, "cx");
        assert_eq!(cur, 1);
    }

    #[test]
    fn typing_combining_mark_keeps_cursor_on_boundary() {
        let (s, cur) = type_str("e\u{301}");
        assert_eq!(cur, s.len());
        assert_eq!(grapheme_col(&s, cur), 1);
    }

    #[test]
    fn stale_mid_char_cursor_is_snapped() {
        let mut s = String::from("ñ");
        let mut cur = 1; // inside the 2-byte 'ñ'
        insert_char(&mut s, &mut cur, 'a');
        assert_eq!(s, "añ");
        assert_eq!(floor_boundary("ñ", 1), 0);
        assert_eq!(next_boundary("ñ", 1), 2);
    }

    #[test]
    fn split_and_width_helpers() {
        let s = "ñ界x";
        let cur = next_boundary(s, 0);
        assert_eq!(split_at_cursor(s, cur), ("ñ", "界", "x"));
        assert_eq!(split_at_cursor(s, s.len()), (s, "", ""));
    }
}
