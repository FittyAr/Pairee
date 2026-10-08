//! Undo/redo for the built-in editor (command pattern).
//!
//! Every modification of the buffer is expressed as a [`TextEdit`] — "at
//! `start`, `removed` was replaced by `inserted`" — which can be applied and
//! reverted. [`EditHistory`] keeps the undo and redo stacks, merges runs of
//! typing into one step and tracks which revision was last saved so the
//! editor knows whether the buffer is dirty.

/// Position in the buffer: line index and byte offset within that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Pos {
    pub y: usize,
    pub x: usize,
}

impl Pos {
    pub fn new(y: usize, x: usize) -> Self {
        Self { y, x }
    }
}

/// Upper bound on undo steps kept in memory.
const MAX_UNDO_STEPS: usize = 10_000;

/// A reversible replacement of text (may span lines via `\n`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: Pos,
    pub removed: String,
    pub inserted: String,
}

/// Position right after `text` when it is laid out starting at `start`.
pub fn end_of(start: Pos, text: &str) -> Pos {
    match text.rfind('\n') {
        None => Pos::new(start.y, start.x + text.len()),
        Some(i) => Pos::new(start.y + text.matches('\n').count(), text.len() - i - 1),
    }
}

/// Replaces the text `removed` found at `start` with `inserted` and returns
/// the position after the inserted text.
fn splice(lines: &mut Vec<String>, start: Pos, removed: &str, inserted: &str) -> Pos {
    let end = end_of(start, removed);
    let mut joined = String::with_capacity(inserted.len() + 64);
    joined.push_str(&lines[start.y][..start.x]);
    joined.push_str(inserted);
    joined.push_str(&lines[end.y][end.x..]);
    let replacement: Vec<String> = joined.split('\n').map(str::to_owned).collect();
    lines.splice(start.y..=end.y, replacement);
    end_of(start, inserted)
}

impl TextEdit {
    /// Applies the edit; returns the cursor position after it.
    pub fn apply(&self, lines: &mut Vec<String>) -> Pos {
        splice(lines, self.start, &self.removed, &self.inserted)
    }

    /// Reverts the edit; returns the cursor position after the restored text.
    pub fn revert(&self, lines: &mut Vec<String>) -> Pos {
        splice(lines, self.start, &self.inserted, &self.removed)
    }

    fn is_typing(&self) -> bool {
        self.removed.is_empty() && !self.inserted.contains('\n')
    }

    fn is_backspacing(&self) -> bool {
        self.inserted.is_empty() && !self.removed.contains('\n')
    }

    /// Merges `next` into `self` when both belong to one run of typing or
    /// of Backspace on the same line.
    fn try_merge(&mut self, next: &TextEdit) -> bool {
        if self.is_typing() && next.is_typing() && next.start == end_of(self.start, &self.inserted)
        {
            self.inserted.push_str(&next.inserted);
            return true;
        }
        if self.is_backspacing()
            && next.is_backspacing()
            && end_of(next.start, &next.removed) == self.start
        {
            self.removed.insert_str(0, &next.removed);
            self.start = next.start;
            return true;
        }
        false
    }
}

#[derive(Debug, Clone)]
struct Step {
    edit: TextEdit,
    cursor_before: Pos,
    revision: u64,
    /// No further edit may be merged into this step.
    sealed: bool,
}

/// Undo/redo stacks plus the saved-revision marker.
#[derive(Debug, Clone, Default)]
pub struct EditHistory {
    undo: Vec<Step>,
    redo: Vec<Step>,
    next_revision: u64,
    current: u64,
    saved: u64,
}

impl EditHistory {
    /// Applies `edit` to `lines`, records it and returns the new cursor.
    pub fn apply(&mut self, lines: &mut Vec<String>, edit: TextEdit, cursor_before: Pos) -> Pos {
        let cursor = edit.apply(lines);
        self.redo.clear();
        self.next_revision += 1;
        self.current = self.next_revision;
        if let Some(last) = self.undo.last_mut()
            && !last.sealed
            && last.edit.try_merge(&edit)
        {
            last.revision = self.current;
            return cursor;
        }
        self.undo.push(Step {
            edit,
            cursor_before,
            revision: self.current,
            sealed: false,
        });
        if self.undo.len() > MAX_UNDO_STEPS {
            self.undo.remove(0);
        }
        cursor
    }

    /// Reverts the last step; returns where the cursor goes.
    pub fn undo(&mut self, lines: &mut Vec<String>) -> Option<Pos> {
        let mut step = self.undo.pop()?;
        step.edit.revert(lines);
        step.sealed = true;
        self.current = self.undo.last().map_or(0, |s| s.revision);
        let cursor = step.cursor_before;
        self.redo.push(step);
        Some(cursor)
    }

    /// Re-applies the last undone step; returns where the cursor goes.
    pub fn redo(&mut self, lines: &mut Vec<String>) -> Option<Pos> {
        let step = self.redo.pop()?;
        let cursor = step.edit.apply(lines);
        self.current = step.revision;
        self.undo.push(step);
        Some(cursor)
    }

    /// Marks the current revision as the one on disk.
    pub fn mark_saved(&mut self) {
        self.saved = self.current;
        if let Some(last) = self.undo.last_mut() {
            last.sealed = true;
        }
    }

    /// Forgets all steps (after reloading the file from disk).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_dirty(&self) -> bool {
        self.current != self.saved
    }

    /// Ends the current typing run so the next edit starts a new undo step.
    pub fn seal(&mut self) {
        if let Some(last) = self.undo.last_mut() {
            last.sealed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn insert(y: usize, x: usize, text: &str) -> TextEdit {
        TextEdit {
            start: Pos::new(y, x),
            removed: String::new(),
            inserted: text.to_string(),
        }
    }

    #[test]
    fn multi_line_edit_applies_and_reverts() {
        let mut buf = lines(&["hello world"]);
        let edit = insert(0, 5, ",\nbig");
        assert_eq!(edit.apply(&mut buf), Pos::new(1, 3));
        assert_eq!(buf, lines(&["hello,", "big world"]));
        assert_eq!(edit.revert(&mut buf), Pos::new(0, 5));
        assert_eq!(buf, lines(&["hello world"]));
    }

    #[test]
    fn typing_run_is_one_undo_step() {
        let mut buf = lines(&[""]);
        let mut h = EditHistory::default();
        for (i, c) in "abc".chars().enumerate() {
            h.apply(&mut buf, insert(0, i, &c.to_string()), Pos::new(0, i));
        }
        assert!(h.is_dirty());
        assert_eq!(h.undo(&mut buf), Some(Pos::new(0, 0)));
        assert_eq!(buf, lines(&[""]));
        assert!(!h.is_dirty());
        assert_eq!(h.redo(&mut buf), Some(Pos::new(0, 3)));
        assert_eq!(buf, lines(&["abc"]));
    }

    #[test]
    fn backspace_run_merges() {
        let mut buf = lines(&["abc"]);
        let mut h = EditHistory::default();
        for x in (0..3).rev() {
            let edit = TextEdit {
                start: Pos::new(0, x),
                removed: buf[0][x..x + 1].to_string(),
                inserted: String::new(),
            };
            h.apply(&mut buf, edit, Pos::new(0, x + 1));
        }
        assert_eq!(buf, lines(&[""]));
        h.undo(&mut buf);
        assert_eq!(buf, lines(&["abc"]));
    }

    #[test]
    fn saved_revision_tracks_dirty_state() {
        let mut buf = lines(&[""]);
        let mut h = EditHistory::default();
        h.apply(&mut buf, insert(0, 0, "a"), Pos::default());
        h.mark_saved();
        assert!(!h.is_dirty());
        h.apply(&mut buf, insert(0, 1, "b"), Pos::new(0, 1));
        assert!(h.is_dirty(), "edits after a save start a new step");
        h.undo(&mut buf);
        assert!(!h.is_dirty());
        assert_eq!(buf, lines(&["a"]));
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut buf = lines(&[""]);
        let mut h = EditHistory::default();
        h.apply(&mut buf, insert(0, 0, "a"), Pos::default());
        h.undo(&mut buf);
        h.apply(&mut buf, insert(0, 0, "b"), Pos::default());
        assert!(h.redo(&mut buf).is_none());
        assert_eq!(buf, lines(&["b"]));
    }
}
