//! Display-width helpers for terminal layout (Unicode-aware).
//!
//! Uses `unicode-width` for column counts and `unicode-segmentation` for
//! grapheme-safe truncation so CJK / emoji names don't break panel columns.
//! Graphemes holding a control character (ESC, newline in a file name…)
//! count as zero columns, as ratatui skips them when drawing.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Display width of `s` in terminal columns.
pub fn display_width(s: &str) -> usize {
    if s.contains(char::is_control) {
        s.graphemes(true).map(grapheme_width).sum()
    } else {
        s.width()
    }
}

/// Columns of one grapheme as drawn (0 for one holding a control char).
fn grapheme_width(g: &str) -> usize {
    if g.contains(char::is_control) {
        0
    } else {
        g.width()
    }
}

/// Truncate `s` so its display width is at most `max_width`.
///
/// When truncation is needed and `max_width >= 1`, appends `…` (one column).
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(s) <= max_width {
        return s.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let budget = max_width - 1; // reserve for ellipsis
    let mut out = String::new();
    let mut used = 0usize;

    for g in s.graphemes(true) {
        let gw = grapheme_width(g);
        if used + gw > budget {
            break;
        }
        out.push_str(g);
        used += gw;
    }
    out.push('…');
    out
}

/// Suffix of `s` after the first `cols` display columns (horizontal
/// scrolling). A wide grapheme cut by the boundary is dropped whole.
pub fn skip_columns(s: &str, cols: usize) -> &str {
    split_at_column(s, cols).1
}

/// Splits `s` before the first grapheme that starts at or after display
/// column `col`; a wide grapheme straddling `col` stays in the first part.
pub fn split_at_column(s: &str, col: usize) -> (&str, &str) {
    let mut used = 0usize;
    for (i, g) in s.grapheme_indices(true) {
        if used >= col {
            return s.split_at(i);
        }
        used += grapheme_width(g);
    }
    (s, "")
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
    for g in s.graphemes(true) {
        if g.starts_with(TAB) {
            let pad = tab_size - col % tab_size;
            out.extend(std::iter::repeat_n(' ', pad));
            col += pad;
        } else {
            out.push_str(g);
            col += grapheme_width(g);
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
    fn skip_columns_scrolls_by_display_width() {
        assert_eq!(skip_columns("hello", 0), "hello");
        assert_eq!(skip_columns("hello", 2), "llo");
        assert_eq!(skip_columns("日本語", 2), "本語");
        assert_eq!(skip_columns("日本語", 1), "本語");
        assert_eq!(skip_columns("ab", 5), "");
    }

    #[test]
    fn split_at_column_keeps_wide_graphemes_whole() {
        assert_eq!(split_at_column("hello", 2), ("he", "llo"));
        assert_eq!(split_at_column("日本語", 3), ("日本", "語"));
        assert_eq!(split_at_column("ab", 9), ("ab", ""));
        assert_eq!(split_at_column("ab", 0), ("", "ab"));
    }

    #[test]
    fn control_chars_take_no_columns() {
        assert_eq!(display_width("a\x1b[31mb"), 6);
        assert_eq!(display_width("a\nb"), 2);
        assert_eq!(truncate_to_width("a\nb", 2), "a\nb");
        assert_eq!(split_at_column("\x1bab", 1), ("\x1ba", "b"));
    }

    #[test]
    fn expand_tabs_measures_graphemes() {
        // A skin-toned emoji is one 2-column grapheme of two chars.
        assert_eq!(expand_tabs("👍🏽\tx", 4), "👍🏽  x");
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
}
