//! Display-column math for editor lines (tabs, wide and combined graphemes).
//!
//! Cursor positions are byte offsets; the screen, mouse clicks and vertical
//! block selections work in display columns. Both directions walk the line
//! grapheme by grapheme with the same widths so they always agree.

use crate::ui::text_width::display_width;
use unicode_segmentation::UnicodeSegmentation;

/// `(byte offset, first column, width)` of every grapheme of `line`.
fn cells(line: &str, tab_size: usize) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
    let tab = tab_size.max(1);
    let mut col = 0usize;
    line.grapheme_indices(true).map(move |(byte, g)| {
        let width = if g == "\t" {
            tab - col % tab
        } else {
            display_width(g)
        };
        let start = col;
        col += width;
        (byte, start, width)
    })
}

/// Display column at which the grapheme starting at byte `byte` is drawn
/// (`byte` past the end yields the width of the whole line).
pub fn byte_to_column(line: &str, byte: usize, tab_size: usize) -> usize {
    let mut end = 0;
    for (b, start, width) in cells(line, tab_size) {
        if b >= byte {
            return start;
        }
        end = start + width;
    }
    end
}

/// Byte offset of the grapheme covering display column `col` (a column in
/// the middle of a wide grapheme maps to its start). Columns past the end of
/// the line map to `line.len()`.
pub fn column_to_byte(line: &str, col: usize, tab_size: usize) -> usize {
    for (b, start, width) in cells(line, tab_size) {
        if col < start + width.max(1) {
            return b;
        }
    }
    line.len()
}

/// Total display width of `line`.
pub fn line_width(line: &str, tab_size: usize) -> usize {
    byte_to_column(line, line.len(), tab_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_tabs() {
        assert_eq!(byte_to_column("a\tb", 1, 4), 1);
        assert_eq!(byte_to_column("a\tb", 2, 4), 4);
        assert_eq!(line_width("a\tb", 4), 5);
        assert_eq!(column_to_byte("a\tb", 2, 4), 1, "inside the tab");
        assert_eq!(column_to_byte("a\tb", 4, 4), 2);
        assert_eq!(column_to_byte("a\tb", 99, 4), 3);
    }

    #[test]
    fn wide_and_combined_graphemes() {
        let line = "日e\u{301}👍🏽x";
        // 日 = 2 cols, é = 1 col (2 chars), 👍🏽 = 2 cols, x = 1 col.
        assert_eq!(byte_to_column(line, 3, 4), 2);
        let thumb = line.find('👍').unwrap();
        assert_eq!(byte_to_column(line, thumb, 4), 3);
        assert_eq!(column_to_byte(line, 1, 4), 0, "middle of 日");
        assert_eq!(column_to_byte(line, 2, 4), 3);
        assert_eq!(column_to_byte(line, 4, 4), thumb, "middle of the emoji");
        assert_eq!(column_to_byte(line, 5, 4), line.len() - 1);
        assert_eq!(line_width(line, 4), 6);
    }

    #[test]
    fn roundtrip_on_boundaries() {
        let line = "ñ\tú日x";
        for (b, _) in line.grapheme_indices(true) {
            assert_eq!(column_to_byte(line, byte_to_column(line, b, 8), 8), b);
        }
    }
}
