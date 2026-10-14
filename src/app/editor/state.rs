//! Buffer state of one built-in editor screen and its editing operations.
//!
//! All modifications go through [`EditorState::edit`], which records them in
//! the undo history. Cursor movement lives in `navigation.rs`.

use super::document::{self, DiskStamp, LoadError, TextFormat};
use super::history::{EditHistory, Pos, TextEdit};
use super::options::EditorOptions;
use super::search::LastSearch;
use super::selection::Selection;
use super::viewport::Viewport;
use crate::app::text_input;
use crate::syntax::{SyntaxCache, SyntaxLimits};
use crate::ui::text_width::{display_width, expand_tabs};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EditorState {
    pub path: PathBuf,
    pub lines: Vec<String>,
    /// Byte offset of the cursor within `lines[cursor_y]`.
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    /// Last find / replace (F3 / Shift+F7 repeat it).
    pub last_search: Option<Box<LastSearch>>,
    /// Highlight the matches of `last_search` (until Esc).
    pub show_matches: bool,
    /// Where the last find stopped on an empty match (`^`, `$`...), which
    /// cannot be shown as a selection; "Replace" replaces it there.
    pub empty_match: Option<Pos>,
    /// Where the last Up / Down left the cursor and the display column it
    /// aims for (kept while the cursor is still there).
    pub goal_column: Option<(Pos, usize)>,
    /// Line ending, final newline and BOM to write back.
    pub format: TextFormat,
    /// Modification time / read-only flag when the file was read or saved.
    pub stamp: DiskStamp,
    /// Edits are refused (read-only file with "Lock editing" on).
    pub locked: bool,
    pub history: EditHistory,
    /// Active selection (anchor; the cursor is the other end).
    pub selection: Option<Selection>,
    /// Columns per tab stop (display, block selection and mouse mapping).
    pub tab_size: usize,
    /// Text area painted last frame, for mouse hit-testing.
    pub viewport: std::cell::Cell<Viewport>,
    /// Block mode (`Ctrl+B`): `Shift`+motions and drags select a vertical
    /// block, for terminals that keep `Alt+Shift`+arrows for themselves.
    pub block_mode: bool,
    /// Set when `path` is a local copy of a file on an SFTP server or in
    /// an archive: saving uploads it (see `editor::remote`).
    pub remote: Option<super::remote::RemoteOrigin>,
    /// Syntax highlighter (`None`: plain text, highlighting off or file too
    /// large). Painting fills its cache, hence the cell.
    pub syntax: RefCell<Option<SyntaxCache>>,
    /// The settings `syntax` was chosen with (kept to choose again after
    /// "save as" or a settings change).
    pub syntax_limits: SyntaxLimits,
}

impl EditorState {
    /// A buffer showing `lines`, cursor at the top.
    fn new(path: PathBuf, lines: Vec<String>, format: TextFormat, tab_size: usize) -> Self {
        Self {
            path,
            lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            last_search: None,
            show_matches: false,
            empty_match: None,
            goal_column: None,
            format,
            stamp: DiskStamp::default(),
            locked: false,
            history: EditHistory::default(),
            selection: None,
            tab_size: tab_size.max(1),
            viewport: Default::default(),
            block_mode: false,
            remote: None,
            syntax: RefCell::new(None),
            syntax_limits: SyntaxLimits::default(),
        }
    }

    /// In-memory buffer for tests.
    #[cfg(test)]
    pub fn from_text(text: &str) -> Self {
        let (lines, format) = document::parse(text);
        Self::new(PathBuf::from("mem.txt"), lines, format, 4)
    }

    /// Opens `path` for editing, applying the tab and read-only options.
    pub fn open(path: PathBuf, options: &EditorOptions) -> Result<Self, LoadError> {
        let loaded = document::load(&path)?;
        let mut state = Self::new(path, loaded.lines, loaded.format, options.tab_size);
        state.stamp = loaded.stamp;
        state.locked = options.lock_read_only && loaded.stamp.read_only;
        state.detect_syntax(&options.syntax);
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

    /// Closing would lose work: edits not saved, or (remote copy) saved
    /// locally but not uploaded to the original yet.
    pub fn has_unsaved_changes(&self) -> bool {
        self.is_dirty() || self.remote.as_ref().is_some_and(|origin| !origin.uploaded)
    }

    /// Applies `edit` through the undo history. Returns `false` when the
    /// buffer is locked.
    pub fn edit(&mut self, edit: TextEdit) -> bool {
        self.edit_group(vec![edit], None)
    }

    /// Applies `edits` as one undo step and drops the selection. The cursor
    /// ends after the last edit unless `cursor_after` says otherwise.
    pub fn edit_group(&mut self, edits: Vec<TextEdit>, cursor_after: Option<Pos>) -> bool {
        if self.locked {
            return false;
        }
        let before = self.cursor();
        if let Some(first) = edits.iter().map(|e| e.start.y).min() {
            self.lines_changed_from(first);
        }
        let after = self.history.apply_group(&mut self.lines, edits, before);
        self.selection = None;
        self.set_cursor(cursor_after.unwrap_or(after));
        true
    }

    /// Inserts `text` at the cursor, replacing the selection if there is one.
    fn insert_at_cursor(&mut self, text: &str) -> bool {
        match self.region() {
            Some(region) => self.replace_region(region, text),
            None => self.edit(TextEdit {
                start: self.cursor(),
                removed: String::new(),
                inserted: text.to_string(),
            }),
        }
    }

    pub fn insert_char(&mut self, c: char) -> bool {
        self.insert_at_cursor(c.encode_utf8(&mut [0; 4]))
    }

    /// Inserts a tab, or spaces up to the next tab stop when tabs expand.
    pub fn insert_tab(&mut self, options: &EditorOptions) -> bool {
        if !options.tab_expansion.inserts_spaces() {
            return self.insert_at_cursor("\t");
        }
        // The width the buffer is shown with (the settings' one since
        // `AppState::apply_settings`).
        let tab = self.tab_size;
        let at = self.insertion_point();
        let before = &self.lines[at.y][..at.x];
        let col = display_width(&expand_tabs(before, tab));
        self.insert_at_cursor(&" ".repeat(tab - col % tab))
    }

    /// Splits the line at the cursor; with auto indent the new line starts
    /// with the leading whitespace of the current one.
    pub fn insert_newline(&mut self, auto_indent: bool) -> bool {
        self.history.seal();
        let mut text = String::from("\n");
        if auto_indent {
            // The line the newline goes into (a selection's start).
            let at = self.insertion_point();
            let line = &self.lines[at.y][..at.x];
            let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            text.push_str(&line[..indent_len]);
        }
        let done = self.insert_at_cursor(&text);
        self.history.seal();
        done
    }

    /// Deletes the grapheme before the cursor, joining lines at column 0.
    pub fn backspace(&mut self) -> bool {
        if self.region().is_some() {
            return self.delete_selection();
        }
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
        if self.region().is_some() {
            return self.delete_selection();
        }
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
        self.time_travel(EditHistory::undo)
    }

    pub fn redo(&mut self) -> bool {
        self.time_travel(EditHistory::redo)
    }

    /// Runs an undo or redo step and moves the cursor where it says.
    fn time_travel(&mut self, step: fn(&mut EditHistory, &mut Vec<String>) -> Option<Pos>) -> bool {
        if self.locked {
            return false;
        }
        let Some(pos) = step(&mut self.history, &mut self.lines) else {
            return false;
        };
        let first = self.history.first_changed_line().unwrap_or(0);
        self.lines_changed_from(first);
        self.selection = None;
        self.set_cursor(pos);
        true
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

    /// Picks the syntax highlighter for the file (none when `limits` say
    /// so or no grammar knows it).
    pub fn detect_syntax(&mut self, limits: &SyntaxLimits) {
        self.syntax_limits = *limits;
        let size: usize = self.lines.iter().map(|l| l.len() + 1).sum();
        let first = self.lines.first().map_or("", String::as_str);
        let cache = limits.cache_for(self.display_path(), first, size as u64);
        *self.syntax.get_mut() = cache;
    }

    /// Lines from `y` on changed: drop what the highlighter knew about them.
    fn lines_changed_from(&mut self, y: usize) {
        if let Some(cache) = self.syntax.get_mut() {
            cache.invalidate_from(y);
        }
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
        if self.path != target {
            // "Save as" with another name may change the language.
            self.path = target.to_path_buf();
            let limits = self.syntax_limits;
            self.detect_syntax(&limits);
        }
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
        self.lines_changed_from(0);
        self.selection = None;
        self.set_cursor(self.cursor());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::TabExpansion;

    fn editor(text: &str) -> EditorState {
        EditorState::from_text(text)
    }

    fn options(tab_expansion: TabExpansion) -> EditorOptions {
        EditorOptions {
            tab_size: 4,
            tab_expansion,
            auto_indent: true,
            lock_read_only: true,
            syntax: SyntaxLimits {
                enabled: true,
                max_bytes: 1024,
            },
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
    fn auto_indent_over_a_selection_uses_the_line_it_starts_on() {
        let mut ed = editor("  a\nbb");
        ed.selection = Some(Selection::Stream {
            anchor: Pos::new(0, 2),
        });
        ed.set_cursor(Pos::new(1, 2));
        ed.insert_newline(true);
        assert_eq!(ed.lines, vec!["  ", "  "]);
    }

    #[test]
    fn up_and_down_keep_the_display_column() {
        let mut ed = editor("日本語\nabcdefgh\nx\n\tabcdefgh");
        ed.set_cursor(Pos::new(0, "日本".len()));
        ed.move_down(1);
        assert_eq!(ed.cursor(), Pos::new(1, 4), "column 4, not byte 6");
        ed.move_down(1);
        assert_eq!(ed.cursor(), Pos::new(2, 1), "end of a short line");
        ed.move_down(1);
        assert_eq!(ed.cursor(), Pos::new(3, 1), "column 4 again (a tab is 4)");
        ed.move_up(3);
        assert_eq!(ed.cursor(), Pos::new(0, "日本".len()));
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
        assert!(ed.syntax.borrow().is_none(), "plain text");
    }

    #[test]
    fn syntax_follows_the_file_type_and_size_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.rs");
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let mut ed = EditorState::open(path, &options(TabExpansion::Keep)).unwrap();
        assert_eq!(ed.syntax.borrow().as_ref().unwrap().language(), "Rust");
        ed.detect_syntax(&SyntaxLimits {
            enabled: true,
            max_bytes: 4,
        });
        assert!(ed.syntax.borrow().is_none(), "over the size limit");
    }

    #[test]
    fn save_as_detects_the_new_language() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(
            &path, "x = 1
",
        )
        .unwrap();
        let mut ed = EditorState::open(path, &options(TabExpansion::Keep)).unwrap();
        assert!(ed.syntax.borrow().is_none(), "plain text");
        ed.save_to(&dir.path().join("script.py")).unwrap();
        assert_eq!(ed.syntax.borrow().as_ref().unwrap().language(), "Python");
        ed.save_to(&dir.path().join("back.txt")).unwrap();
        assert!(ed.syntax.borrow().is_none());
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
