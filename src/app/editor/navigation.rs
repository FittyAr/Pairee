//! Cursor movement and scrolling for [`EditorState`].

use super::columns::{byte_to_column, column_to_byte};
use super::history::Pos;
use super::state::EditorState;
use crate::app::text_input;

impl EditorState {
    pub fn move_up(&mut self, rows: usize) {
        self.move_vertically(self.cursor_y.saturating_sub(rows));
    }

    pub fn move_down(&mut self, rows: usize) {
        self.move_vertically(self.cursor_y.saturating_add(rows));
    }

    /// Moves to line `y` keeping the display column (tabs and wide
    /// characters included). A run of vertical moves keeps the column it
    /// started from, across shorter lines.
    fn move_vertically(&mut self, y: usize) {
        let here = self.cursor();
        let column = match self.goal_column {
            Some((at, column)) if at == here => column,
            _ => byte_to_column(self.current_line(), self.cursor_x, self.tab_size),
        };
        let y = y.min(self.lines.len().saturating_sub(1));
        let x = column_to_byte(&self.lines[y], column, self.tab_size);
        self.set_cursor(Pos::new(y, x));
        self.goal_column = Some((self.cursor(), column));
    }

    /// One grapheme left, wrapping to the end of the previous line.
    pub fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x = text_input::prev_boundary(self.current_line(), self.cursor_x);
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.current_line().len();
        }
    }

    /// One grapheme right, wrapping to the start of the next line.
    pub fn move_right(&mut self) {
        if self.cursor_x < self.current_line().len() {
            self.cursor_x = text_input::next_boundary(self.current_line(), self.cursor_x);
        } else if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    pub fn move_line_start(&mut self) {
        self.cursor_x = 0;
    }

    pub fn move_line_end(&mut self) {
        self.cursor_x = self.current_line().len();
    }

    pub fn move_doc_start(&mut self) {
        self.set_cursor(Pos::default());
    }

    pub fn move_doc_end(&mut self) {
        let y = self.lines.len().saturating_sub(1);
        self.set_cursor(Pos::new(y, usize::MAX));
    }

    /// Scrolls just enough to keep the cursor inside a `height`-row view.
    pub fn ensure_cursor_visible(&mut self, height: usize) {
        let height = height.max(1);
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y + 1 - height;
        }
    }
}
