//! Display-width helpers for terminal layout (Unicode-aware).
//!
//! Uses `unicode-width` for column counts and `unicode-segmentation` for
//! grapheme-safe truncation so CJK / emoji names don't break panel columns.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Display width of `s` in terminal columns.
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Truncate `s` so its display width is at most `max_width`.
///
/// When truncation is needed and `max_width >= 1`, appends `…` (one column).
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if s.width() <= max_width {
        return s.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let budget = max_width - 1; // reserve for ellipsis
    let mut out = String::new();
    let mut used = 0usize;

    for g in s.graphemes(true) {
        let gw = g.width();
        if used + gw > budget {
            break;
        }
        out.push_str(g);
        used += gw;
    }
    out.push('…');
    out
}

/// Pad or truncate `s` to exactly `width` display columns.
#[allow(dead_code)] // public helper for fixed-width columns
pub fn pad_or_truncate(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let w = s.width();
    if w == width {
        s.to_string()
    } else if w > width {
        truncate_to_width(s, width)
    } else {
        let mut out = s.to_string();
        out.push_str(&" ".repeat(width - w));
        out
    }
}

/// Width of a single Unicode scalar, treating control chars as 0.
#[allow(dead_code)]
pub fn char_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

const TAB: char = '\t';

/// Replaces each tab with spaces up to the next multiple of `tab_size` columns
/// (editor / viewer `*_tab_size` settings). A size of 0 is treated as 1.
pub fn expand_tabs(s: &str, tab_size: usize) -> String {
    if !s.contains(TAB) {
        return s.to_string();
    }
    let tab_size = tab_size.max(1);
    let mut out = String::with_capacity(s.len());
    let mut col = 0usize;
    for c in s.chars() {
        if c == TAB {
            let pad = tab_size - col % tab_size;
            out.extend(std::iter::repeat_n(' ', pad));
            col += pad;
        } else {
            out.push(c);
            col += UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tabs_aligns_to_tab_stops() {
        assert_eq!(expand_tabs("a\tb", 4), "a   b");
        assert_eq!(expand_tabs("\tx", 2), "  x");
        assert_eq!(expand_tabs("abcd\te", 4), "abcd    e");
        assert_eq!(expand_tabs("no tabs", 8), "no tabs");
        assert_eq!(expand_tabs("\t", 0), " ");
    }

    #[test]
    fn ascii_width() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn cjk_double_width() {
        assert_eq!(display_width("日本語"), 6);
    }

    #[test]
    fn truncate_ascii_with_ellipsis() {
        assert_eq!(truncate_to_width("hello world", 8), "hello w…");
        assert_eq!(truncate_to_width("hi", 10), "hi");
        assert_eq!(truncate_to_width("abc", 0), "");
        assert_eq!(truncate_to_width("abc", 1), "…");
    }

    #[test]
    fn truncate_cjk() {
        // 日=2 本=2 語=2; max 5 → budget 4 for content → "日本…" (2+2+1=5)
        assert_eq!(truncate_to_width("日本語", 5), "日本…");
        // max 3 → budget 2 → "日…" (2+1=3)
        assert_eq!(truncate_to_width("日本語", 3), "日…");
        assert_eq!(truncate_to_width("日本語", 6), "日本語");
    }

    #[test]
    fn pad_or_truncate_exact() {
        assert_eq!(pad_or_truncate("ab", 5), "ab   ");
        assert_eq!(pad_or_truncate("abcdef", 4), "abc…");
    }
}
