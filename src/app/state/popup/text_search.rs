use crate::app::form::{FormKey, FormLayout};
use crate::app::text_input::TextField;
use crossterm::event::KeyEvent;

/// Find dialog shared by the viewer and the editor: query, "case sensitive",
/// [Search] and [Cancel].
#[derive(Debug, Clone, Default)]
pub struct TextSearchState {
    pub query: TextField,
    pub case_sensitive: bool,
    pub cursor_idx: usize,
}

/// What a key in the find dialog asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKey {
    /// Run the search (Enter on any row but Cancel).
    Find,
    /// Esc or Cancel.
    Close,
    /// Edited / moved / toggled; keep the dialog.
    Stay,
}

impl TextSearchState {
    pub const FORM: FormLayout = FormLayout::with_input(4, 2);
    pub const ROW_CASE: usize = 1;
    pub const BUTTON_CANCEL: usize = 3;

    pub fn handle_key(&mut self, key: &KeyEvent) -> SearchKey {
        match Self::FORM.handle(&mut self.cursor_idx, Some(&mut self.query), key) {
            FormKey::Toggle(Self::ROW_CASE) => {
                self.case_sensitive = !self.case_sensitive;
                SearchKey::Stay
            }
            FormKey::Activate(Self::BUTTON_CANCEL) | FormKey::Cancel => SearchKey::Close,
            FormKey::Activate(_) => SearchKey::Find,
            FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => SearchKey::Stay,
        }
    }

    /// The non-empty query and its case flag (for match highlighting).
    pub fn active_query(&self) -> Option<(&str, bool)> {
        (!self.query.is_empty()).then(|| (self.query.text(), self.case_sensitive))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn typing_toggling_and_buttons() {
        let mut s = TextSearchState::default();
        assert_eq!(s.handle_key(&key(KeyCode::Char('a'))), SearchKey::Stay);
        assert_eq!(s.active_query(), Some(("a", false)));
        s.handle_key(&key(KeyCode::Tab));
        s.handle_key(&key(KeyCode::Char(' ')));
        assert!(s.case_sensitive);
        assert_eq!(s.handle_key(&key(KeyCode::Enter)), SearchKey::Find);
        s.handle_key(&key(KeyCode::Up));
        s.handle_key(&key(KeyCode::Up));
        assert_eq!(s.cursor_idx, TextSearchState::BUTTON_CANCEL);
        assert_eq!(s.handle_key(&key(KeyCode::Enter)), SearchKey::Close);
    }
}
