//! Text mode cursor and selection of the viewer: a cursor line moved with
//! the arrows (or put by a click), whole lines selected from it with
//! Shift+arrows, Ctrl+A or by dragging the mouse, and copied to the
//! clipboard (Ctrl+C) without opening the editor. With nothing selected,
//! Ctrl+C copies the cursor line.
//!
//! Lines are the unit because the viewer wraps long lines and pages huge
//! files from disk: a line range is cheap to keep and to read back.

use super::state::ViewerState;
use ratatui::layout::Rect;
use std::ops::RangeInclusive;

/// Most bytes put on the clipboard at once (Ctrl+A on a huge log): the
/// copy stops at the line that passes it.
pub const MAX_COPY_BYTES: usize = 32 * 1024 * 1024;
/// Lines read from the document per step while copying.
const COPY_CHUNK: usize = 4096;

/// Selected lines: where the selection started and the line it was
/// extended to (either may be the larger one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSelection {
    pub anchor: usize,
    pub end: usize,
}

impl LineSelection {
    pub fn lines(&self) -> RangeInclusive<usize> {
        self.anchor.min(self.end)..=self.anchor.max(self.end)
    }

    pub fn contains(&self, line: usize) -> bool {
        self.lines().contains(&line)
    }
}

/// Where text mode painted its lines last frame, for the mouse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextLayout {
    /// The text area (inside the frame).
    pub area: Rect,
    /// Each painted line: its number, first screen row and rows taken
    /// (more than one when wrapped).
    pub rows: Vec<(usize, u16, u16)>,
}

impl TextLayout {
    /// The line painted at screen row `row`: past the top, the line above
    /// the first one shown (the selection then scrolls up), past the
    /// bottom, the one below the last; `None` when nothing was painted.
    pub fn line_at(&self, row: u16) -> Option<usize> {
        let first = self.rows.first()?;
        let last = self.rows.last()?;
        if row < self.area.y {
            return Some(first.0.saturating_sub(1));
        }
        if row >= self.area.bottom() {
            return Some(last.0 + 1);
        }
        let line = self
            .rows
            .iter()
            .find(|(_, top, height)| row >= *top && row < top + height)
            .map_or(last.0, |(line, ..)| *line);
        Some(line)
    }

    pub fn contains(&self, column: u16, row: u16) -> bool {
        self.area
            .contains(ratatui::layout::Position::new(column, row))
    }
}

/// What a copy put on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub text: String,
    pub lines: usize,
    /// Stopped at [`MAX_COPY_BYTES`].
    pub truncated: bool,
}

impl ViewerState {
    /// Moves the cursor line by `delta`. With `extend` (Shift) the
    /// selection grows from where the cursor was to where it lands;
    /// without, the selection goes. The cursor stays on screen (`height`
    /// text rows).
    pub fn move_cursor(&mut self, delta: isize, height: usize, extend: bool) {
        let line = self.cursor.saturating_add_signed(delta);
        self.cursor_to(line, height, extend);
    }

    /// Puts the cursor on `line` (see [`Self::move_cursor`]).
    pub fn cursor_to(&mut self, line: usize, height: usize, extend: bool) {
        let line = line.min(self.last_scroll());
        if extend {
            let anchor = self.selection.map_or(self.cursor, |s| s.anchor);
            self.selection = Some(LineSelection { anchor, end: line });
        } else {
            self.selection = None;
        }
        self.cursor = line;
        self.reveal_line(line, height);
    }

    /// Text rows shown last frame (a page when nothing was painted yet).
    pub fn text_rows(&self) -> usize {
        match self.layout.borrow().area.height {
            0 => 20,
            rows => usize::from(rows),
        }
    }

    /// Selects every line indexed so far (the cursor goes to the last).
    pub fn select_all(&mut self) {
        let last = self.last_scroll();
        self.selection = Some(LineSelection {
            anchor: 0,
            end: last,
        });
        self.cursor = last;
    }

    /// The lines a copy takes: the selection, else the cursor line.
    pub fn copy_range(&self) -> RangeInclusive<usize> {
        self.selection
            .map_or(self.cursor..=self.cursor, |s| s.lines())
    }

    /// Scrolls so `line` is shown whole. Wrapped lines take several rows:
    /// what the last frame painted says which lines fitted.
    fn reveal_line(&mut self, line: usize, height: usize) {
        if line < self.scroll {
            self.scroll = line;
            return;
        }
        let fitted = {
            let layout = self.layout.borrow();
            let painted_here = layout.rows.first().is_some_and(|r| r.0 == self.scroll);
            painted_here
                .then(|| {
                    layout
                        .rows
                        .iter()
                        .rfind(|(_, top, rows)| top + rows <= layout.area.bottom())
                        .map(|r| r.0)
                })
                .flatten()
        };
        let last_shown = fitted.unwrap_or(self.scroll + height.max(1) - 1);
        if line > last_shown {
            self.scroll += line - last_shown;
        }
    }

    /// The lines of [`Self::copy_range`] as text (`\n` between them), at
    /// most [`MAX_COPY_BYTES`].
    pub fn selected_text(&self) -> Copied {
        let range = self.copy_range();
        let (first, last) = (*range.start(), *range.end());
        let mut text = String::new();
        let mut lines = 0;
        let mut next = first;
        while next <= last {
            let count = COPY_CHUNK.min(last - next + 1);
            let chunk = self.doc.lines(next as u64, count);
            if chunk.is_empty() {
                break;
            }
            for line in &chunk {
                if lines > 0 {
                    text.push('\n');
                }
                if text.len() + line.len() > MAX_COPY_BYTES {
                    return Copied {
                        text,
                        lines,
                        truncated: true,
                    };
                }
                text.push_str(line);
                lines += 1;
            }
            next += chunk.len();
        }
        Copied {
            text,
            lines,
            truncated: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn viewer(lines: usize) -> ViewerState {
        let text: Vec<String> = (0..lines).map(|i| format!("line {i}")).collect();
        ViewerState::from_text(PathBuf::from("v.txt"), text)
    }

    #[test]
    fn the_cursor_moves_and_shift_selects_from_it() {
        let mut vw = viewer(50);
        vw.cursor_to(10, 5, false);
        assert_eq!(vw.scroll, 6, "the cursor is kept on screen");
        assert_eq!(vw.copy_range(), 10..=10, "no selection: the cursor line");
        vw.move_cursor(2, 5, true);
        assert_eq!(vw.selection.unwrap().lines(), 10..=12);
        vw.move_cursor(-5, 5, true);
        assert_eq!(vw.selection.unwrap().lines(), 7..=10, "past the anchor");
        assert_eq!(vw.cursor, 7);
        let copied = vw.selected_text();
        assert_eq!(copied.text, "line 7\nline 8\nline 9\nline 10");
        assert_eq!((copied.lines, copied.truncated), (4, false));
        vw.move_cursor(1, 5, false);
        assert!(vw.selection.is_none(), "a plain move drops the selection");
        assert_eq!(vw.cursor, 8);
    }

    #[test]
    fn select_all_copies_every_line() {
        let mut vw = viewer(3);
        assert_eq!(vw.selected_text().text, "line 0");
        vw.select_all();
        assert_eq!(vw.selected_text().text, "line 0\nline 1\nline 2");
        assert_eq!(vw.cursor, 2);
    }

    #[test]
    fn a_drag_selects_lines_and_scrolls_past_the_edges() {
        let mut vw = viewer(100);
        let layout = TextLayout {
            area: Rect::new(1, 1, 20, 3),
            rows: vec![(40, 1, 1), (41, 2, 2)],
        };
        assert_eq!(layout.line_at(1), Some(40));
        assert_eq!(layout.line_at(3), Some(41), "second row of a wrapped line");
        assert_eq!(layout.line_at(0), Some(39));
        assert_eq!(layout.line_at(9), Some(42));
        vw.scroll = 40;
        vw.cursor_to(41, 3, false);
        vw.cursor_to(39, 3, true);
        assert_eq!(vw.selection.unwrap().lines(), 39..=41);
        assert_eq!(vw.scroll, 39);
    }

    #[test]
    fn wrapped_lines_scroll_by_what_fitted() {
        let mut vw = viewer(100);
        vw.scroll = 10;
        *vw.layout.borrow_mut() = TextLayout {
            area: Rect::new(0, 0, 10, 4),
            // Line 11 wraps over three rows: line 12 did not fit.
            rows: vec![(10, 0, 1), (11, 1, 3)],
        };
        vw.cursor_to(12, 4, false);
        assert_eq!(vw.scroll, 11);
    }
}
