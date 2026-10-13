//! Find / replace dialog of the built-in editor (F7 / Ctrl+F7).
//!
//! One dialog in two shapes: "find" has the pattern, the options and
//! [Find] [Cancel]; "replace" adds the replacement field and [Replace]
//! [All]. The rows are described by [`FindRow`] lists, so focus handling,
//! painting and mouse hits all follow the same table.

use crate::app::editor::search::{LastSearch, Matcher, SearchOptions};
use crate::app::form::{FormKey, FormLayout};
use crate::app::text_input::TextField;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A checkbox of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFlag {
    CaseSensitive,
    WholeWords,
    Regex,
    Backward,
    Wrap,
}

impl SearchFlag {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::CaseSensitive => "sys_case_sensitive",
            Self::WholeWords => "editor_find_whole_words",
            Self::Regex => "editor_find_regex",
            Self::Backward => "editor_find_backward",
            Self::Wrap => "editor_find_wrap",
        }
    }

    fn slot(self, options: &mut SearchOptions) -> &mut bool {
        match self {
            Self::CaseSensitive => &mut options.case_sensitive,
            Self::WholeWords => &mut options.whole_words,
            Self::Regex => &mut options.regex,
            Self::Backward => &mut options.backward,
            Self::Wrap => &mut options.wrap,
        }
    }

    pub fn get(self, options: SearchOptions) -> bool {
        let mut options = options;
        *self.slot(&mut options)
    }
}

/// A button of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindButton {
    Find,
    Replace,
    ReplaceAll,
    Cancel,
}

impl FindButton {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Find => "btn_find_bracket",
            Self::Replace => "btn_replace_bracket",
            Self::ReplaceAll => "btn_replace_all_bracket",
            Self::Cancel => "btn_cancel_bracket",
        }
    }
}

/// One focusable row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindRow {
    Query,
    Replacement,
    Flag(SearchFlag),
    Button(FindButton),
}

use FindButton as B;
use FindRow as R;
use SearchFlag as F;

const FIND_ROWS: [FindRow; 8] = [
    R::Query,
    R::Flag(F::CaseSensitive),
    R::Flag(F::WholeWords),
    R::Flag(F::Regex),
    R::Flag(F::Backward),
    R::Flag(F::Wrap),
    R::Button(B::Find),
    R::Button(B::Cancel),
];

const REPLACE_ROWS: [FindRow; 11] = [
    R::Query,
    R::Replacement,
    R::Flag(F::CaseSensitive),
    R::Flag(F::WholeWords),
    R::Flag(F::Regex),
    R::Flag(F::Backward),
    R::Flag(F::Wrap),
    R::Button(B::Find),
    R::Button(B::Replace),
    R::Button(B::ReplaceAll),
    R::Button(B::Cancel),
];

/// What a key in the dialog asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindKey {
    /// Edited, moved or toggled: keep the dialog.
    Stay,
    Close,
    Run(FindButton),
}

/// State of the find / replace dialog.
#[derive(Debug, Clone, Default)]
pub struct EditorSearchPrompt {
    pub query: TextField,
    /// The replacement text; `None` in the "find" shape.
    pub replacement: Option<TextField>,
    pub options: SearchOptions,
    pub cursor_idx: usize,
}

impl EditorSearchPrompt {
    pub fn rows(&self) -> &'static [FindRow] {
        if self.replacement.is_some() {
            &REPLACE_ROWS
        } else {
            &FIND_ROWS
        }
    }

    pub fn form(&self) -> FormLayout {
        let rows = self.rows();
        let first_button = rows
            .iter()
            .position(|r| matches!(r, R::Button(_)))
            .unwrap_or(rows.len());
        FormLayout::new(rows.len(), first_button)
    }

    fn focused_row(&self) -> FindRow {
        self.rows()
            .get(self.cursor_idx)
            .copied()
            .unwrap_or(R::Query)
    }

    /// The text field of the focused row, if it is one.
    pub fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        match self.focused_row() {
            R::Query => Some(&mut self.query),
            R::Replacement => self.replacement.as_mut(),
            R::Flag(_) | R::Button(_) => None,
        }
    }

    /// The button Enter presses outside the button bar.
    fn default_button(&self) -> FindButton {
        if self.replacement.is_some() {
            B::Replace
        } else {
            B::Find
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> FindKey {
        let form = self.form();
        let mut focus = self.cursor_idx;
        let outcome = form.handle(&mut focus, self.focused_field_mut(), key);
        self.cursor_idx = focus;
        match outcome {
            FormKey::Toggle(row) => {
                if let Some(R::Flag(flag)) = self.rows().get(row) {
                    let slot = flag.slot(&mut self.options);
                    *slot = !*slot;
                }
                FindKey::Stay
            }
            FormKey::Activate(row) => match self.rows().get(row) {
                Some(R::Button(B::Cancel)) => FindKey::Close,
                Some(R::Button(button)) => FindKey::Run(*button),
                _ => FindKey::Run(self.default_button()),
            },
            FormKey::Cancel => FindKey::Close,
            FormKey::Handled | FormKey::Other => FindKey::Stay,
        }
    }

    /// The compiled pattern, if it is valid and not empty (for highlighting
    /// matches while typing).
    pub fn matcher(&self) -> Option<Matcher> {
        Matcher::new(self.query.text(), self.options).ok().flatten()
    }

    /// The search to run: `Ok(None)` for an empty pattern, the regex error
    /// for an invalid one.
    pub fn to_search(&self) -> Result<Option<LastSearch>, regex::Error> {
        let matcher = Matcher::new(self.query.text(), self.options)?;
        Ok(matcher.map(|matcher| LastSearch {
            pattern: self.query.text().to_string(),
            replacement: self.replacement.as_ref().map(|f| f.text().to_string()),
            options: self.options,
            matcher,
        }))
    }

    /// `Some(older)` for the history keys of the text fields: Ctrl/Alt+Up
    /// (older entry) and Ctrl/Alt+Down (newer one).
    pub fn history_step(key: &KeyEvent) -> Option<bool> {
        let modified = key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Up if modified => Some(true),
            KeyCode::Down if modified => Some(false),
            _ => None,
        }
    }

    /// Replaces the focused field with the history entry (newest first)
    /// older or newer than its text: `searches` for the pattern,
    /// `replacements` for the replacement.
    pub fn recall(&mut self, searches: &[String], replacements: &[String], older: bool) {
        let history = if self.focused_row() == R::Replacement {
            replacements
        } else {
            searches
        };
        let Some(field) = self.focused_field_mut() else {
            return;
        };
        let at = history.iter().position(|h| h == field.text());
        let next = match (at, older) {
            (None, true) => Some(0),
            (Some(i), true) => Some((i + 1).min(history.len().saturating_sub(1))),
            (Some(i), false) => i.checked_sub(1),
            (None, false) => None,
        };
        if let Some(text) = next.and_then(|i| history.get(i)) {
            field.set_text(text.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn find_shape_toggles_options_and_runs() {
        let mut d = EditorSearchPrompt::default();
        assert_eq!(d.handle_key(&key(KeyCode::Char('x'))), FindKey::Stay);
        assert_eq!(d.query.text(), "x");
        d.handle_key(&key(KeyCode::Down));
        d.handle_key(&key(KeyCode::Down));
        d.handle_key(&key(KeyCode::Down));
        d.handle_key(&key(KeyCode::Char(' ')));
        assert!(d.options.regex);
        assert_eq!(d.handle_key(&key(KeyCode::Enter)), FindKey::Run(B::Find));
        d.handle_key(&key(KeyCode::Up));
        d.handle_key(&key(KeyCode::Up));
        d.handle_key(&key(KeyCode::Up));
        d.handle_key(&key(KeyCode::Up));
        assert_eq!(d.cursor_idx, 7);
        assert_eq!(d.handle_key(&key(KeyCode::Enter)), FindKey::Close);
    }

    #[test]
    fn replace_shape_has_both_fields_and_buttons() {
        let mut d = EditorSearchPrompt {
            replacement: Some(TextField::default()),
            ..Default::default()
        };
        assert_eq!(d.form().first_button, 7);
        d.handle_key(&key(KeyCode::Tab));
        d.handle_key(&key(KeyCode::Char('r')));
        assert_eq!(d.replacement.as_ref().unwrap().text(), "r");
        assert_eq!(d.handle_key(&key(KeyCode::Enter)), FindKey::Run(B::Replace));
        d.cursor_idx = 9;
        assert_eq!(
            d.handle_key(&key(KeyCode::Enter)),
            FindKey::Run(B::ReplaceAll)
        );
    }

    #[test]
    fn invalid_regex_is_reported_and_not_highlighted() {
        let mut d = EditorSearchPrompt::default();
        d.query.set_text("(a");
        assert!(d.to_search().unwrap().is_some(), "plain text: fine");
        d.options.regex = true;
        assert!(d.to_search().is_err());
        assert!(d.matcher().is_none());
    }

    #[test]
    fn history_recall_walks_older_and_newer() {
        let history = vec!["new".to_string(), "mid".into(), "old".into()];
        let mut d = EditorSearchPrompt::default();
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        assert_eq!(EditorSearchPrompt::history_step(&ctrl_up), Some(true));
        assert_eq!(EditorSearchPrompt::history_step(&key(KeyCode::Up)), None);
        d.recall(&history, &[], true);
        assert_eq!(d.query.text(), "new");
        d.recall(&history, &[], true);
        d.recall(&history, &[], true);
        d.recall(&history, &[], true);
        assert_eq!(d.query.text(), "old", "stops at the oldest");
        d.recall(&history, &[], false);
        assert_eq!(d.query.text(), "mid");
    }
}
