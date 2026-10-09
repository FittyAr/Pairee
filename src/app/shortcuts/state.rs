//! State of the keyboard-shortcuts modal.

use super::model::{ContextTab, ShortcutRow};
use crate::app::text_input::TextField;
use crate::config::localization::t;

/// What the modal is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Listing and filtering.
    Browse,
    /// Recording the keys of a new chord for the cursor row; `add` keeps
    /// the row's other chords.
    Capture { add: bool, keys: Vec<String> },
    /// The captured chord is bound to `owner`: Enter replaces, Esc cancels.
    ConfirmReplace {
        add: bool,
        chord: String,
        owner: String,
    },
    /// The next key pressed becomes the key filter.
    KeySearch,
    /// Name of the preset file to write.
    Export { name: TextField },
    /// Drop every override of the previewed preset (Enter) or not (Esc).
    ConfirmResetAll,
}

#[derive(Debug, Clone)]
pub struct ShortcutsState {
    pub tab: ContextTab,
    pub query: TextField,
    /// Show only rows bound to this chord (search by key).
    pub key_filter: Option<String>,
    /// Index among the visible rows.
    pub cursor: usize,
    /// Preset shown (the active one unless previewing another).
    pub preset: String,
    pub rows: Vec<ShortcutRow>,
    pub mode: Mode,
    /// One-line feedback (saved, exported, fragile key...).
    pub message: Option<String>,
}

impl ShortcutsState {
    pub fn new(preset: String, rows: Vec<ShortcutRow>) -> Self {
        Self {
            tab: ContextTab::Panels,
            query: TextField::default(),
            key_filter: None,
            cursor: 0,
            preset,
            rows,
            mode: Mode::Browse,
            message: None,
        }
    }

    /// Rows of the current tab that pass the text or key filter.
    pub fn visible(&self) -> Vec<&ShortcutRow> {
        let query = self.query.text().trim().to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.tab == self.tab)
            .filter(|r| match &self.key_filter {
                Some(chord) => r.chords.iter().any(|c| c == chord),
                None => query.is_empty() || matches_query(r, &query),
            })
            .collect()
    }

    /// The row under the cursor.
    pub fn current(&self) -> Option<&ShortcutRow> {
        self.visible().get(self.cursor).copied()
    }

    /// Keeps the cursor on a visible row after the list changed.
    pub fn clamp_cursor(&mut self) {
        let len = self.visible().len();
        self.cursor = self.cursor.min(len.saturating_sub(1));
    }

    /// Filters by `chord`, showing the first tab where it is bound.
    pub fn search_key(&mut self, chord: String) {
        if let Some(tab) = ContextTab::ALL.iter().copied().find(|tab| {
            self.rows
                .iter()
                .any(|r| r.tab == *tab && r.chords.contains(&chord))
        }) {
            self.tab = tab;
        }
        self.message = Some(t("shortcuts_key_filter").replace("{}", &chord));
        self.key_filter = Some(chord);
        self.cursor = 0;
    }

    /// The row of the current tab, other than `except`, that a new chord
    /// would collide with (same chord, or one the start of the other).
    pub fn owner_of(&self, chord: &str, except: &str) -> Option<&ShortcutRow> {
        let seq = chord.parse::<keybinds::KeySeq>().ok()?;
        self.rows
            .iter()
            .filter(|r| r.tab == self.tab && r.id != except)
            .find(|r| {
                r.chords.iter().any(|c| {
                    c.parse::<keybinds::KeySeq>()
                        .is_ok_and(|other| crate::keybindings::chord::seqs_overlap(&seq, &other))
                })
            })
    }
}

fn matches_query(row: &ShortcutRow, query: &str) -> bool {
    row.label.to_lowercase().contains(query)
        || row.id.to_lowercase().contains(query)
        || row.chords.iter().any(|c| c.to_lowercase().contains(query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::catalog::Category;

    fn row(tab: ContextTab, id: &str, chords: &[&str]) -> ShortcutRow {
        ShortcutRow {
            tab,
            id: id.into(),
            label: id.replace('_', " "),
            category: Category::Files,
            chords: chords.iter().map(|c| c.to_string()).collect(),
            user: false,
            plugin: None,
            conflict: false,
            fragile: false,
            run: None,
        }
    }

    fn state() -> ShortcutsState {
        ShortcutsState::new(
            "norton".into(),
            vec![
                row(ContextTab::Panels, "copy", &["F5"]),
                row(ContextTab::Panels, "go_to_top", &["g g"]),
                row(ContextTab::Panels, "move", &["F6"]),
                row(ContextTab::Editor, "save", &["F2", "Ctrl+s"]),
            ],
        )
    }

    #[test]
    fn text_filter_matches_labels_ids_and_chords() {
        let mut s = state();
        s.query.set_text("f6");
        assert_eq!(s.visible().len(), 1);
        s.query.set_text("cop");
        assert_eq!(s.current().unwrap().id, "copy");
    }

    #[test]
    fn key_search_switches_to_the_tab_that_binds_it() {
        let mut s = state();
        s.search_key("Ctrl+s".into());
        assert_eq!(s.tab, ContextTab::Editor);
        assert_eq!(s.current().unwrap().id, "save");
    }

    #[test]
    fn owners_include_sequence_prefixes() {
        let s = state();
        assert_eq!(s.owner_of("F5", "move").unwrap().id, "copy");
        assert_eq!(s.owner_of("g", "move").unwrap().id, "go_to_top");
        assert!(
            s.owner_of("F5", "copy").is_none(),
            "its own chord is no conflict"
        );
        assert!(s.owner_of("F9", "copy").is_none());
    }
}
