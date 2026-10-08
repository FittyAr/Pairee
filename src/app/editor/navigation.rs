//! Cursor movement and scrolling for [`EditorState`].

use super::history::Pos;
use super::state::EditorState;
use crate::app::sys_helpers::find_next_in_editor;
use crate::app::text_input;

impl EditorState {
    pub fn move_up(&mut self, rows: usize) {
        let y = self.cursor_y.saturating_sub(rows);
        self.set_cursor(Pos::new(y, self.cursor_x));
    }

    pub fn move_down(&mut self, rows: usize) {
        let y = self.cursor_y.saturating_add(rows);
        self.set_cursor(Pos::new(y, self.cursor_x));
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

    /// Moves to the next match of `query` and centres it when off-screen.
    /// Returns `false` when there is no match.
    pub fn find_next(&mut self, query: &str, case_sensitive: bool, height: usize) -> bool {
        let Some((x, y)) = find_next_in_editor(
            &self.lines,
            self.cursor_x,
            self.cursor_y,
            query,
            case_sensitive,
        ) else {
            return false;
        };
        self.set_cursor(Pos::new(y, x));
        if self.cursor_y < self.scroll_y || self.cursor_y >= self.scroll_y + height.max(1) {
            self.scroll_y = self.cursor_y.saturating_sub(height / 2);
        }
        self.last_search = Some(query.to_string());
        self.last_case_sensitive = case_sensitive;
        true
    }

    /// Repeats the last search, if any.
    pub fn repeat_search(&mut self, height: usize) -> bool {
        match self.last_search.clone() {
            Some(q) => self.find_next(&q, self.last_case_sensitive, height),
            None => false,
        }
    }
}
