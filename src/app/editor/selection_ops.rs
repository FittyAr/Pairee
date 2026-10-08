//! Selection, clipboard-facing and paste operations of [`EditorState`].

use super::columns::{byte_to_column, column_to_byte};
use super::history::Pos;
use super::selection::{Region, Selection};
use super::state::EditorState;

/// How a cursor motion treats the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extend {
    /// Plain motion: the selection is dropped.
    No,
    /// `Shift+motion`: extend a stream selection.
    Stream,
    /// `Alt+Shift+motion`: extend a vertical block selection.
    Block,
}

/// A cursor motion (the keys that move without editing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Up(usize),
    Down(usize),
    Left,
    Right,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
}

impl EditorState {
    /// The current non-empty selection, normalized.
    pub fn region(&self) -> Option<Region> {
        self.selection.and_then(|s| s.region(self.cursor()))
    }

    pub fn has_selection(&self) -> bool {
        self.region().is_some()
    }

    /// Display column of the cursor.
    pub fn cursor_column(&self) -> usize {
        byte_to_column(self.current_line(), self.cursor_x, self.tab_size)
    }

    /// Where typed text goes: the start of the selection, or the cursor.
    pub fn insertion_point(&self) -> Pos {
        self.region().map_or(self.cursor(), |r| {
            r.insertion_point(&self.lines, self.tab_size)
        })
    }

    /// Moves the cursor, extending or dropping the selection.
    pub fn move_cursor(&mut self, motion: Motion, extend: Extend) {
        match extend {
            Extend::No => self.selection = None,
            Extend::Stream => self.anchor_stream(),
            Extend::Block => return self.move_block_head(motion),
        }
        match motion {
            Motion::Up(n) => self.move_up(n),
            Motion::Down(n) => self.move_down(n),
            Motion::Left => self.move_left(),
            Motion::Right => self.move_right(),
            Motion::LineStart => self.move_line_start(),
            Motion::LineEnd => self.move_line_end(),
            Motion::DocStart => self.move_doc_start(),
            Motion::DocEnd => self.move_doc_end(),
        }
    }

    /// Starts a stream selection at the cursor unless one is active.
    pub fn anchor_stream(&mut self) {
        if !matches!(self.selection, Some(Selection::Stream { .. })) {
            self.selection = Some(Selection::Stream {
                anchor: self.cursor(),
            });
        }
    }

    /// Block motions move the head by display columns, so the rectangle
    /// keeps its shape across short lines and tabs.
    fn move_block_head(&mut self, motion: Motion) {
        let cursor_col = self.cursor_column();
        let (anchor_y, anchor_col, mut head_col) = match self.selection {
            Some(Selection::Block {
                anchor_y,
                anchor_col,
                head_col,
            }) => (anchor_y, anchor_col, head_col),
            _ => (self.cursor_y, cursor_col, cursor_col),
        };
        let mut y = self.cursor_y;
        match motion {
            Motion::Up(n) => y = y.saturating_sub(n),
            Motion::Down(n) => y = (y + n).min(self.lines.len().saturating_sub(1)),
            Motion::Left => head_col = head_col.saturating_sub(1),
            Motion::Right => head_col += 1,
            Motion::LineStart => head_col = 0,
            Motion::LineEnd => head_col = super::columns::line_width(&self.lines[y], self.tab_size),
            Motion::DocStart => y = 0,
            Motion::DocEnd => y = self.lines.len().saturating_sub(1),
        }
        self.cursor_y = y;
        self.cursor_x = column_to_byte(&self.lines[y], head_col, self.tab_size);
        self.selection = Some(Selection::Block {
            anchor_y,
            anchor_col,
            head_col,
        });
    }

    /// Selects the whole buffer (`Ctrl+A`).
    pub fn select_all(&mut self) {
        self.selection = Some(Selection::Stream {
            anchor: Pos::default(),
        });
        self.move_doc_end();
    }

    /// The selected text, if any.
    pub fn selected_text(&self) -> Option<String> {
        self.region().map(|r| r.text(&self.lines, self.tab_size))
    }

    /// Removes the selected text as one undo step.
    pub fn delete_selection(&mut self) -> bool {
        match self.region() {
            Some(region) => self.replace_region(region, ""),
            None => false,
        }
    }

    /// Replaces `region` with `text` as one undo step; the cursor ends after
    /// the inserted text.
    pub fn replace_region(&mut self, region: Region, text: &str) -> bool {
        let at = region.insertion_point(&self.lines, self.tab_size);
        let mut edits = region.removal_edits(&self.lines, self.tab_size);
        if !text.is_empty() {
            edits.push(super::history::TextEdit {
                start: at,
                removed: String::new(),
                inserted: text.to_string(),
            });
        }
        self.history.seal();
        let after = super::history::end_of(at, text);
        let done = self.edit_group(edits, Some(after));
        self.history.seal();
        done
    }

    /// Inserts pasted `text` (any line ending) as one undo step, replacing
    /// the selection.
    pub fn insert_text(&mut self, text: &str) -> bool {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if text.is_empty() {
            return false;
        }
        let region = self.region().unwrap_or(Region::Stream {
            start: self.cursor(),
            end: self.cursor(),
        });
        self.replace_region(region, &text)
    }

    /// Returns the selected text and removes it (`Ctrl+X`). `None` when
    /// nothing is selected or the buffer is locked.
    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        self.delete_selection().then_some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &str) -> EditorState {
        EditorState::from_text(text)
    }

    #[test]
    fn shift_motion_selects_graphemes() {
        let mut e = ed("ñ👍🏽e\u{301}x");
        e.move_cursor(Motion::Right, Extend::Stream);
        e.move_cursor(Motion::Right, Extend::Stream);
        e.move_cursor(Motion::Right, Extend::Stream);
        assert_eq!(e.selected_text().as_deref(), Some("ñ👍🏽e\u{301}"));
        e.move_cursor(Motion::Left, Extend::Stream);
        assert_eq!(e.selected_text().as_deref(), Some("ñ👍🏽"));
        e.move_cursor(Motion::Right, Extend::No);
        assert!(!e.has_selection());
    }

    #[test]
    fn multi_line_selection_and_select_all() {
        let mut e = ed("abc\ndef\nghi");
        e.cursor_x = 1;
        e.move_cursor(Motion::Down(1), Extend::Stream);
        assert_eq!(e.selected_text().as_deref(), Some("bc\nd"));
        e.select_all();
        assert_eq!(e.selected_text().as_deref(), Some("abc\ndef\nghi"));
    }

    #[test]
    fn typing_replaces_selection_in_one_undo_step() {
        let mut e = ed("hello world");
        e.cursor_x = 6;
        e.move_cursor(Motion::LineEnd, Extend::Stream);
        assert!(e.insert_char('X'));
        assert_eq!(e.lines, vec!["hello X"]);
        assert!(e.undo());
        assert_eq!(e.lines, vec!["hello world"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn cut_and_paste_undo() {
        let mut e = ed("one\ntwo\nthree");
        e.move_cursor(Motion::Down(1), Extend::Stream);
        let cut = e.cut_selection().unwrap();
        assert_eq!(cut, "one\n");
        assert_eq!(e.lines, vec!["two", "three"]);
        e.move_cursor(Motion::DocEnd, Extend::No);
        assert!(e.insert_text("\r\nX\r\nY"));
        assert_eq!(e.lines, vec!["two", "three", "X", "Y"]);
        assert_eq!(e.cursor(), Pos::new(3, 1));
        assert!(e.undo(), "paste is one step");
        assert_eq!(e.lines, vec!["two", "three"]);
        assert!(e.undo());
        assert_eq!(e.lines, vec!["one", "two", "three"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut e = ed("abcdef");
        e.cursor_x = 1;
        e.move_cursor(Motion::Right, Extend::Stream);
        e.move_cursor(Motion::Right, Extend::Stream);
        assert!(e.backspace());
        assert_eq!(e.lines, vec!["adef"]);
        assert_eq!(e.cursor(), Pos::new(0, 1));
    }

    #[test]
    fn block_selection_copy_and_delete() {
        let mut e = ed("abcdef\nab\n12345\n日本語");
        e.cursor_x = 1;
        e.move_cursor(Motion::Right, Extend::Block);
        e.move_cursor(Motion::Right, Extend::Block);
        e.move_cursor(Motion::Down(3), Extend::Block);
        assert_eq!(e.cursor(), Pos::new(3, 3), "column 3 is 本");
        assert_eq!(e.selected_text().as_deref(), Some("bc\nb\n23\n日"));
        assert!(e.delete_selection());
        assert_eq!(e.lines, vec!["adef", "a", "145", "本語"]);
        assert_eq!(e.cursor(), Pos::new(0, 1));
        assert!(e.undo(), "block delete is one step");
        assert_eq!(e.lines, vec!["abcdef", "ab", "12345", "日本語"]);
    }

    #[test]
    fn block_head_survives_short_lines() {
        let mut e = ed("abcdef\nx\nabcdef");
        e.move_cursor(Motion::LineEnd, Extend::Block);
        e.move_cursor(Motion::Down(1), Extend::Block);
        assert_eq!(
            e.cursor(),
            Pos::new(1, 1),
            "cursor clamps on the short line"
        );
        e.move_cursor(Motion::Down(1), Extend::Block);
        assert_eq!(e.cursor(), Pos::new(2, 6), "head column kept");
        assert_eq!(e.selected_text().as_deref(), Some("abcdef\nx\nabcdef"));
    }

    #[test]
    fn typing_over_block_inserts_at_top_left() {
        let mut e = ed("abc\nabc");
        e.cursor_x = 1;
        e.move_cursor(Motion::Right, Extend::Block);
        e.move_cursor(Motion::Down(1), Extend::Block);
        assert!(e.insert_char('Z'));
        assert_eq!(e.lines, vec!["aZc", "ac"]);
        assert_eq!(e.cursor(), Pos::new(0, 2));
    }

    #[test]
    fn locked_buffer_keeps_selection_text() {
        let mut e = ed("ro");
        e.locked = true;
        e.select_all();
        assert!(e.cut_selection().is_none());
        assert_eq!(e.lines, vec!["ro"]);
        assert_eq!(e.selected_text().as_deref(), Some("ro"));
    }
}
