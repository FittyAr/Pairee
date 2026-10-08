//! Text selection model of the built-in editor.
//!
//! A selection is an *anchor* plus the cursor (the head). Stream selections
//! cover the text between two positions; vertical block selections (Far
//! style, `Alt+Shift+arrows`) cover a rectangle of display columns over a
//! range of lines, where the head column may lie past the end of a line.

use super::columns::column_to_byte;
use super::history::{Pos, TextEdit};

/// Selection anchor; the other end is always the editor cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Stream {
        anchor: Pos,
    },
    Block {
        anchor_y: usize,
        anchor_col: usize,
        /// Display column of the head (may exceed the cursor line width).
        head_col: usize,
    },
}

/// The selected area resolved against the cursor, normalized (start ≤ end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Stream {
        start: Pos,
        end: Pos,
    },
    /// Lines `top..=bottom`, display columns `left..right`.
    Block {
        top: usize,
        bottom: usize,
        left: usize,
        right: usize,
    },
}

impl Selection {
    /// Resolves the selection against the cursor. Empty selections yield
    /// `None`.
    pub fn region(&self, cursor: Pos) -> Option<Region> {
        match *self {
            Self::Stream { anchor } => {
                let (start, end) = (anchor.min(cursor), anchor.max(cursor));
                (start != end).then_some(Region::Stream { start, end })
            }
            Self::Block {
                anchor_y,
                anchor_col,
                head_col,
            } => (anchor_col != head_col).then_some(Region::Block {
                top: anchor_y.min(cursor.y),
                bottom: anchor_y.max(cursor.y),
                left: anchor_col.min(head_col),
                right: anchor_col.max(head_col),
            }),
        }
    }
}

impl Region {
    /// Byte range of line `y` covered by the region (`None` outside it).
    pub fn span_on_line(&self, y: usize, line: &str, tab_size: usize) -> Option<(usize, usize)> {
        match *self {
            Self::Stream { start, end } if (start.y..=end.y).contains(&y) => {
                let from = if y == start.y { start.x } else { 0 };
                let to = if y == end.y { end.x } else { line.len() };
                Some((from, to))
            }
            Self::Block {
                top,
                bottom,
                left,
                right,
            } if (top..=bottom).contains(&y) => Some((
                column_to_byte(line, left, tab_size),
                column_to_byte(line, right, tab_size),
            )),
            _ => None,
        }
    }

    /// `true` when the region continues past the end of line `y` (the line
    /// break itself is selected).
    pub fn includes_line_break(&self, y: usize) -> bool {
        matches!(*self, Self::Stream { start, end } if start.y <= y && y < end.y)
    }

    /// Where text typed over the region is inserted.
    pub fn insertion_point(&self, lines: &[String], tab_size: usize) -> Pos {
        match *self {
            Self::Stream { start, .. } => start,
            Self::Block { top, left, .. } => {
                Pos::new(top, column_to_byte(&lines[top], left, tab_size))
            }
        }
    }

    /// The selected text, covered lines joined with `\n`.
    pub fn text(&self, lines: &[String], tab_size: usize) -> String {
        self.line_spans(lines, tab_size)
            .map(|(y, from, to)| &lines[y][from..to])
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Edits that remove the region, ordered so they can be applied in
    /// sequence (each one leaves the positions of the following ones valid).
    pub fn removal_edits(&self, lines: &[String], tab_size: usize) -> Vec<TextEdit> {
        match *self {
            Self::Stream { start, .. } => vec![TextEdit {
                start,
                removed: self.text(lines, tab_size),
                inserted: String::new(),
            }],
            Self::Block { .. } => self
                .line_spans(lines, tab_size)
                .filter(|(_, from, to)| from < to)
                .map(|(y, from, to)| TextEdit {
                    start: Pos::new(y, from),
                    removed: lines[y][from..to].to_string(),
                    inserted: String::new(),
                })
                .collect(),
        }
    }

    /// `(line, from, to)` byte spans of every covered line.
    fn line_spans<'a>(
        &'a self,
        lines: &'a [String],
        tab_size: usize,
    ) -> impl Iterator<Item = (usize, usize, usize)> + 'a {
        let (top, bottom) = match *self {
            Self::Stream { start, end } => (start.y, end.y),
            Self::Block { top, bottom, .. } => (top, bottom),
        };
        (top..=bottom.min(lines.len().saturating_sub(1))).filter_map(move |y| {
            self.span_on_line(y, &lines[y], tab_size)
                .map(|(from, to)| (y, from, to))
        })
    }
}
