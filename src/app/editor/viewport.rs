//! Screen geometry of the editor text area, recorded while painting so mouse
//! events can be mapped back to buffer positions without the UI layer.

use super::columns::column_to_byte;
use super::history::Pos;
use super::selection::Selection;
use super::state::EditorState;

/// Text area painted in the last frame (terminal cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    /// Left column of the text (after the line-number gutter).
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// Display columns scrolled out on the left.
    pub scroll_x: usize,
}

impl Viewport {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        (self.x..self.x.saturating_add(self.width)).contains(&col)
            && (self.y..self.y.saturating_add(self.height)).contains(&row)
    }
}

impl EditorState {
    /// Buffer position under terminal cell `(col, row)`. Cells outside the
    /// text area are clamped to its edges (dragging past the border).
    pub fn pos_at_cell(&self, col: u16, row: u16) -> Pos {
        let (y, display_col) = self.line_and_column_at_cell(col, row);
        Pos::new(
            y,
            column_to_byte(&self.lines[y], display_col, self.tab_size),
        )
    }

    /// Line index and display column under terminal cell `(col, row)`,
    /// clamped like [`Self::pos_at_cell`]; the column may lie past the end
    /// of the line (block selections).
    pub fn line_and_column_at_cell(&self, col: u16, row: u16) -> (usize, usize) {
        let vp = self.viewport.get();
        let last_row = vp.y.saturating_add(vp.height.saturating_sub(1));
        let row = row.clamp(vp.y, last_row.max(vp.y));
        let y = (self.scroll_y + usize::from(row - vp.y)).min(self.lines.len().saturating_sub(1));
        (y, vp.scroll_x + usize::from(col.saturating_sub(vp.x)))
    }

    /// Left button pressed on a cell: moves the cursor there and anchors a
    /// new (still empty) selection — a vertical block one when `block`.
    pub fn mouse_press(&mut self, col: u16, row: u16, block: bool) {
        let (y, display_col) = self.line_and_column_at_cell(col, row);
        self.set_cursor(self.pos_at_cell(col, row));
        self.selection = Some(if block {
            Selection::Block {
                anchor_y: y,
                anchor_col: display_col,
                head_col: display_col,
            }
        } else {
            Selection::Stream {
                anchor: self.cursor(),
            }
        });
    }

    /// Mouse dragged to a cell: moves the selection head there.
    pub fn mouse_drag(&mut self, col: u16, row: u16) {
        let (_, display_col) = self.line_and_column_at_cell(col, row);
        self.set_cursor(self.pos_at_cell(col, row));
        match &mut self.selection {
            Some(Selection::Block { head_col, .. }) => *head_col = display_col,
            Some(Selection::Stream { .. }) => {}
            None => self.anchor_stream(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cells_to_positions() {
        let ed = EditorState::from_text("a\tb\n日本\nlast");
        ed.viewport.set(Viewport {
            x: 10,
            y: 2,
            width: 20,
            height: 5,
            scroll_x: 0,
        });
        assert_eq!(ed.pos_at_cell(10, 2), Pos::new(0, 0));
        assert_eq!(ed.pos_at_cell(13, 2), Pos::new(0, 1), "inside the tab");
        assert_eq!(ed.pos_at_cell(14, 2), Pos::new(0, 2));
        assert_eq!(ed.pos_at_cell(13, 3), Pos::new(1, 3), "second half of 日");
        assert_eq!(ed.pos_at_cell(29, 4), Pos::new(2, 4), "past the end");
        assert_eq!(ed.pos_at_cell(0, 0), Pos::new(0, 0), "clamped above");
        assert_eq!(ed.pos_at_cell(10, 40), Pos::new(2, 0), "clamped below");
        assert!(ed.viewport.get().contains(29, 6));
        assert!(!ed.viewport.get().contains(30, 6));
    }

    fn with_viewport(text: &str) -> EditorState {
        let ed = EditorState::from_text(text);
        ed.viewport.set(Viewport {
            x: 0,
            y: 0,
            width: 40,
            height: 10,
            scroll_x: 0,
        });
        ed
    }

    #[test]
    fn drag_selects_stream_text() {
        let mut ed = with_viewport("hello\nworld");
        ed.mouse_press(1, 0, false);
        assert!(!ed.has_selection());
        ed.mouse_drag(3, 1);
        assert_eq!(ed.selected_text().as_deref(), Some("ello\nwor"));
    }

    #[test]
    fn alt_drag_selects_block_past_line_ends() {
        let mut ed = with_viewport("abcdef\nab\nabcdef");
        ed.mouse_press(1, 0, true);
        ed.mouse_drag(4, 2);
        assert_eq!(ed.selected_text().as_deref(), Some("bcd\nb\nbcd"));
    }
}
