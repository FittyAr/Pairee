//! [`TextField`]: the single-line, grapheme-aware text box used by every
//! prompt (mkdir, rename, copy/move, filters, Git and SSH dialogs...).
//!
//! Key handling lives here once: typing, Backspace/Delete, Left/Right,
//! Home/End and `Ctrl+V` paste. The UI side renders it with
//! `crate::ui::text_field`.

use super::{backspace, delete, floor_boundary, insert_char, next_boundary, prev_boundary};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Result of feeding a key to a [`TextField`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldEdit {
    /// The text changed.
    Edited,
    /// Only the cursor moved.
    Moved,
    /// The key is not a text-editing key; the caller may handle it.
    Ignored,
}

impl FieldEdit {
    /// `true` unless the key was ignored.
    pub fn consumed(self) -> bool {
        self != Self::Ignored
    }
}

/// A single-line text input with a cursor kept on grapheme boundaries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// Byte offset, always on a grapheme boundary.
    cursor: usize,
}

impl TextField {
    /// A field holding `text` with the cursor at the end.
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Cursor byte offset into [`Self::text`].
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn into_text(self) -> String {
        self.text
    }

    /// Replaces the text and moves the cursor to the end.
    pub fn set_text(&mut self, text: impl Into<String>) {
        *self = Self::new(text);
    }

    pub fn clear(&mut self) {
        self.set_text(String::new());
    }

    /// `true` when the cursor is after the last grapheme.
    pub fn cursor_at_end(&self) -> bool {
        self.cursor >= self.text.len()
    }

    pub fn insert_char(&mut self, c: char) {
        insert_char(&mut self.text, &mut self.cursor, c);
    }

    /// Inserts `s` at the cursor (no newline filtering; see [`Self::paste`]).
    pub fn insert_str(&mut self, s: &str) {
        let at = floor_boundary(&self.text, self.cursor);
        self.text.insert_str(at, s);
        self.cursor = floor_boundary(&self.text, at + s.len());
    }

    /// Inserts the first non-empty line of a paste payload. Returns `true`
    /// when something was inserted.
    pub fn paste(&mut self, raw: &str) -> bool {
        let line = first_paste_line(raw);
        if line.is_empty() {
            return false;
        }
        self.insert_str(&line);
        true
    }

    pub fn backspace(&mut self) -> bool {
        backspace(&mut self.text, &mut self.cursor)
    }

    pub fn delete(&mut self) -> bool {
        delete(&mut self.text, &mut self.cursor)
    }

    pub fn move_left(&mut self) {
        self.cursor = prev_boundary(&self.text, self.cursor);
    }

    pub fn move_right(&mut self) {
        self.cursor = next_boundary(&self.text, self.cursor);
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Applies a text-editing key: printable characters, Backspace, Delete,
    /// Left/Right, Home/End and `Ctrl+V` (paste from the system clipboard).
    pub fn handle_key(&mut self, key: &KeyEvent) -> FieldEdit {
        match key.code {
            KeyCode::Char('v' | 'V') if is_plain_ctrl(key.modifiers) => {
                let pasted = crate::app::sys_helpers::clipboard::get_text()
                    .map(|raw| self.paste(&raw))
                    .unwrap_or(false);
                if pasted {
                    FieldEdit::Edited
                } else {
                    FieldEdit::Moved
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                FieldEdit::Edited
            }
            KeyCode::Backspace => edited_or_moved(self.backspace()),
            KeyCode::Delete => edited_or_moved(self.delete()),
            KeyCode::Left => self.moved(Self::move_left),
            KeyCode::Right => self.moved(Self::move_right),
            KeyCode::Home => self.moved(Self::move_home),
            KeyCode::End => self.moved(Self::move_end),
            _ => FieldEdit::Ignored,
        }
    }

    /// Like [`Self::handle_key`] but only for the keys that change the text
    /// (characters, Backspace, Delete, `Ctrl+V`); used by fields whose
    /// dialog binds the arrows / Home / End to something else.
    pub fn handle_edit_key(&mut self, key: &KeyEvent) -> FieldEdit {
        match key.code {
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => self.handle_key(key),
            _ => FieldEdit::Ignored,
        }
    }

    fn moved(&mut self, step: fn(&mut Self)) -> FieldEdit {
        step(self);
        FieldEdit::Moved
    }
}

impl From<String> for TextField {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for TextField {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl std::fmt::Display for TextField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// `Ctrl` without `Alt` (`Ctrl+Alt` is AltGr on Windows and types characters).
fn is_plain_ctrl(mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT)
}

fn edited_or_moved(changed: bool) -> FieldEdit {
    if changed {
        FieldEdit::Edited
    } else {
        FieldEdit::Moved
    }
}

/// First non-empty line of a paste payload (`\r` stripped).
pub fn first_paste_line(raw: &str) -> String {
    raw.replace('\r', "")
        .lines()
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_into(field: &mut TextField, text: &str) {
        for c in text.chars() {
            field.handle_key(&key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn typing_and_cursor_keys() {
        let mut field = TextField::default();
        type_into(&mut field, "ñandú");
        assert_eq!(field.text(), "ñandú");
        field.handle_key(&key(KeyCode::Home));
        field.handle_key(&key(KeyCode::Right));
        type_into(&mut field, "X");
        assert_eq!(field.text(), "ñXandú");
        field.handle_key(&key(KeyCode::End));
        assert_eq!(
            field.handle_key(&key(KeyCode::Backspace)),
            FieldEdit::Edited
        );
        assert_eq!(field.text(), "ñXand");
        field.handle_key(&key(KeyCode::Home));
        assert_eq!(field.handle_key(&key(KeyCode::Delete)), FieldEdit::Edited);
        assert_eq!(field.text(), "Xand");
        assert_eq!(field.handle_key(&key(KeyCode::Enter)), FieldEdit::Ignored);
    }

    #[test]
    fn altgr_characters_are_typed() {
        let mut field = TextField::default();
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        field.handle_key(&KeyEvent::new(KeyCode::Char('@'), altgr));
        assert_eq!(field.text(), "@");
    }

    #[test]
    fn paste_inserts_first_line_at_cursor() {
        let mut field = TextField::new("ab");
        field.move_left();
        assert!(field.paste("\r\nXY\r\nignored"));
        assert_eq!(field.text(), "aXYb");
        assert_eq!(field.cursor(), 3);
        assert!(!field.paste("\n\n"));
    }

    #[test]
    fn edit_keys_leave_arrows_to_the_caller() {
        let mut field = TextField::new("ab");
        assert_eq!(
            field.handle_edit_key(&key(KeyCode::Left)),
            FieldEdit::Ignored
        );
        assert!(field.cursor_at_end());
        assert!(field.handle_edit_key(&key(KeyCode::Backspace)).consumed());
        assert_eq!(field.text(), "a");
    }

    #[test]
    fn first_paste_line_strips_cr_and_extra_lines() {
        assert_eq!(first_paste_line("hello\r\nworld"), "hello");
        assert_eq!(first_paste_line("\n\nfoo"), "foo");
        assert_eq!(first_paste_line(""), "");
    }
}
