//! Buffer state of one built-in editor screen and its editing operations.
//!
//! All modifications go through [`EditorState::edit`], which records them in
//! the undo history. Cursor movement lives in `navigation.rs`.

use super::document::{self, DiskStamp, LoadError, TextFormat};
use super::history::{EditHistory, Pos, TextEdit};
use super::options::EditorOptions;
use crate::app::text_input;
use crate::ui::text_width::{display_width, expand_tabs};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EditorState {
    pub path: PathBuf,
    pub lines: Vec<String>,
    /// Byte offset of the cursor within `lines[cursor_y]`.
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub last_search: Option<String>,
    pub last_case_sensitive: bool,
    /// Line ending, final newline and BOM to write back.
    pub format: TextFormat,
    /// Modification time / read-only flag when the file was read or saved.
    pub stamp: DiskStamp,
    /// Edits are refused (read-only file with "Lock editing" on).
    pub locked: bool,
    pub history: EditHistory,
}

impl EditorState {
    /// Opens `path` for editing, applying the tab and read-only options.
    pub fn open(path: PathBuf, options: &EditorOptions) -> Result<Self, LoadError> {
        let loaded = document::load(&path)?;
        let mut state = Self {
            path,
            lines: loaded.lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            last_search: None,
            last_case_sensitive: false,
            format: loaded.format,
            stamp: loaded.stamp,
            locked: options.lock_read_only && loaded.stamp.read_only,
            history: EditHistory::default(),
        };
        if options.tab_expansion == crate::config::settings::TabExpansion::ConvertAll
            && !state.locked
        {
            state.convert_tabs(options.tab_size);
        }
        Ok(state)
    }

    pub fn cursor(&self) -> Pos {
        Pos::new(self.cursor_y, self.cursor_x)
    }

    pub fn set_cursor(&mut self, pos: Pos) {
        self.cursor_y = pos.y.min(self.lines.len().saturating_sub(1));
        let line = self.current_line();
        self.cursor_x = text_input::floor_boundary(line, pos.x);
    }

    pub fn current_line(&self) -> &str {
        self.lines.get(self.cursor_y).map_or("", String::as_str)
    }

    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty()
    }

    /// Applies `edit` through the undo history. Returns `false` when the
    /// buffer is locked.
    pub fn edit(&mut self, edit: TextEdit) -> bool {
        if self.locked {
            return false;
        }
        let before = self.cursor();
        let after = self.history.apply(&mut self.lines, edit, before);
        self.set_cursor(after);
        true
    }

    fn insert_at_cursor(&mut self, text: &str) -> bool {
        self.edit(TextEdit {
            start: self.cursor(),
            removed: String::new(),
            inserted: text.to_string(),
        })
    }

    pub fn insert_char(&mut self, c: char) -> bool {
        self.insert_at_cursor(c.encode_utf8(&mut [0; 4]))
    }

    /// Inserts a tab, or spaces up to the next tab stop when tabs expand.
    pub fn insert_tab(&mut self, options: &EditorOptions) -> bool {
        if !options.tab_expansion.inserts_spaces() {
            return self.insert_at_cursor("\t");
        }
        let tab = options.tab_size.max(1);
        let before = &self.current_line()[..self.cursor_x];
        let col = display_width(&expand_tabs(before, tab));
        self.insert_at_cursor(&" ".repeat(tab - col % tab))
    }

    /// Splits the line at the cursor; with auto indent the new line starts
    /// with the leading whitespace of the current one.
    pub fn insert_newline(&mut self, auto_indent: bool) -> bool {
        self.history.seal();
        let mut text = String::from("\n");
        if auto_indent {
            let line = &self.current_line()[..self.cursor_x];
            let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            text.push_str(&line[..indent_len]);
        }
        let done = self.insert_at_cursor(&text);
        self.history.seal();
        done
    }

    /// Deletes the grapheme before the cursor, joining lines at column 0.
    pub fn backspace(&mut self) -> bool {
        let pos = self.cursor();
        let start = if pos.x > 0 {
            Pos::new(pos.y, text_input::prev_boundary(self.current_line(), pos.x))
        } else if pos.y > 0 {
            Pos::new(pos.y - 1, self.lines[pos.y - 1].len())
        } else {
            return false;
        };
        self.remove_range(start, pos)
    }

    /// Deletes the grapheme after the cursor, joining lines at the end.
    pub fn delete_forward(&mut self) -> bool {
        let pos = self.cursor();
        let line = self.current_line();
        let end = if pos.x < line.len() {
            Pos::new(pos.y, text_input::next_boundary(line, pos.x))
        } else if pos.y + 1 < self.lines.len() {
            Pos::new(pos.y + 1, 0)
        } else {
            return false;
        };
        self.history.seal();
        let done = self.remove_range(pos, end);
        self.history.seal();
        done
    }

    fn remove_range(&mut self, start: Pos, end: Pos) -> bool {
        let removed = if start.y == end.y {
            self.lines[start.y][start.x..end.x].to_string()
        } else {
            format!(
                "{}\n{}",
                &self.lines[start.y][start.x..],
                &self.lines[end.y][..end.x]
            )
        };
        self.edit(TextEdit {
            start,
            removed,
            inserted: String::new(),
        })
    }

    pub fn undo(&mut self) -> bool {
        if self.locked {
            return false;
        }
        match self.history.undo(&mut self.lines) {
            Some(pos) => {
                self.set_cursor(pos);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.locked {
            return false;
        }
        match self.history.redo(&mut self.lines) {
            Some(pos) => {
                self.set_cursor(pos);
                true
            }
            None => false,
        }
    }

    /// Replaces every tab in the buffer with spaces (one undo step per line).
    fn convert_tabs(&mut self, tab_size: usize) {
        for y in 0..self.lines.len() {
            if self.lines[y].contains('\t') {
                self.edit(TextEdit {
                    start: Pos::new(y, 0),
                    removed: self.lines[y].clone(),
                    inserted: expand_tabs(&self.lines[y], tab_size),
                });
                self.history.seal();
            }
        }
        self.set_cursor(Pos::default());
    }

    /// `true` when the file on disk changed since it was read or saved.
    pub fn changed_on_disk(&self) -> bool {
        let now = DiskStamp::of(&self.path);
        now.modified.is_some() && now.modified != self.stamp.modified
    }

    /// Writes the buffer to `target` (the current path or a "save as" path)
    /// and makes it the edited file.
    pub fn save_to(&mut self, target: &Path) -> anyhow::Result<()> {
        self.stamp = document::save(target, &self.lines, &self.format)?;
        self.path = target.to_path_buf();
        self.locked = false;
        self.history.mark_saved();
        Ok(())
    }

    /// Re-reads the file from disk, discarding unsaved changes and history.
    pub fn reload(&mut self) -> Result<(), LoadError> {
        let loaded = document::load(&self.path)?;
        self.lines = loaded.lines;
        self.format = loaded.format;
        self.stamp = loaded.stamp;
        self.history.reset();
        self.set_cursor(self.cursor());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::TabExpansion;

    fn editor(text: &str) -> EditorState {
        let (lines, format) = document::parse(text);
        EditorState {
            path: PathBuf::from("mem.txt"),
            lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            last_search: None,
            last_case_sensitive: false,
            format,
            stamp: DiskStamp::default(),
            locked: false,
            history: EditHistory::default(),
        }
    }

    fn options(tab_expansion: TabExpansion) -> EditorOptions {
        EditorOptions {
            tab_size: 4,
            tab_expansion,
            auto_indent: true,
            lock_read_only: true,
        }
    }

    #[test]
    fn auto_indent_copies_leading_whitespace() {
        let mut ed = editor("\t  code");
        ed.cursor_x = ed.lines[0].len();
        ed.insert_newline(true);
        assert_eq!(ed.lines, vec!["\t  code", "\t  "]);
        assert_eq!(ed.cursor(), Pos::new(1, 3));
        ed.insert_newline(false);
        assert_eq!(ed.lines[2], "");
    }

    #[test]
    fn tab_expands_to_next_stop() {
        let mut ed = editor("ab");
        ed.cursor_x = 2;
        ed.insert_tab(&options(TabExpansion::NewTabs));
        assert_eq!(ed.lines[0], "ab  ");
        ed.insert_tab(&options(TabExpansion::Keep));
        assert_eq!(ed.lines[0], "ab  \t");
    }

    #[test]
    fn backspace_and_delete_join_lines_and_undo() {
        let mut ed = editor("one\ntwo");
        ed.cursor_y = 1;
        assert!(ed.backspace());
        assert_eq!(ed.lines, vec!["onetwo"]);
        assert_eq!(ed.cursor(), Pos::new(0, 3));
        assert!(ed.delete_forward());
        assert_eq!(ed.lines, vec!["onewo"]);
        assert!(ed.undo());
        assert!(ed.undo());
        assert_eq!(ed.lines, vec!["one", "two"]);
        assert!(!ed.is_dirty());
        assert!(ed.redo());
        assert_eq!(ed.lines, vec!["onetwo"]);
    }

    #[test]
    fn locked_buffer_refuses_edits() {
        let mut ed = editor("ro");
        ed.locked = true;
        assert!(!ed.insert_char('x'));
        assert!(!ed.backspace());
        assert_eq!(ed.lines, vec!["ro"]);
        assert!(!ed.is_dirty());
    }

    #[test]
    fn open_converts_tabs_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.txt");
        std::fs::write(&path, "\tx\r\n").unwrap();
        let ed = EditorState::open(path, &options(TabExpansion::ConvertAll)).unwrap();
        assert_eq!(ed.lines, vec!["    x"]);
        assert!(ed.is_dirty());
        assert_eq!(ed.format.line_ending, document::LineEnding::CrLf);
    }

    #[test]
    fn save_marks_clean_and_detects_external_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.txt");
        std::fs::write(&path, "a\n").unwrap();
        let mut ed = EditorState::open(path.clone(), &options(TabExpansion::Keep)).unwrap();
        ed.insert_char('b');
        assert!(ed.is_dirty());
        ed.save_to(&path).unwrap();
        assert!(!ed.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ba\n");
        assert!(!ed.changed_on_disk());
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(later).unwrap();
        assert!(ed.changed_on_disk());
    }
}
