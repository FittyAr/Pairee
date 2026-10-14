//! Tagging panel entries, and the one rule of what each way of tagging may
//! take (setting "Select folders", `select_folders`, as in Far Manager):
//!
//! * One entry at a time (Insert / Space, Ctrl+click, a right click) always
//!   toggles the entry under the cursor, folder or not: the user pointed at
//!   it.
//! * Many entries at once (Gray `+` / `-` masks, Select all, Gray `*`
//!   invert, Shift+click and right-drag ranges, visual mode) take folders
//!   only when "Select folders" is on.
//!
//! `..` is never tagged. Every path goes through [`takes_single`] or
//! [`takes_group`].

use super::panel::PanelState;
use crate::fs::FileEntry;
use crate::fs::glob::glob_matches;
use std::collections::HashSet;
use std::path::PathBuf;

/// Whether a one-entry toggle may tag `entry`.
pub fn takes_single(entry: &FileEntry) -> bool {
    entry.name != ".."
}

/// Whether a selection of many entries at once may tag or untag `entry`:
/// folders only with `select_folders`.
pub fn takes_group(entry: &FileEntry, select_folders: bool) -> bool {
    takes_single(entry) && (select_folders || !entry.is_dir)
}

impl PanelState {
    /// Toggles the tag of the entry under the cursor.
    pub fn toggle_selection(&mut self) {
        let idx = self.cursor_index;
        let selected = !self.is_selected(idx);
        self.set_selected(idx, selected);
    }

    /// Tags (`selected`) or untags entry `idx` alone ([`takes_single`]).
    pub fn set_selected(&mut self, idx: usize, selected: bool) {
        if self.entries.get(idx).is_some_and(takes_single) {
            self.mark(idx, selected);
        }
    }

    /// Tags or untags entry `idx` as part of a group ([`takes_group`]).
    pub fn set_group_selected(&mut self, idx: usize, selected: bool, select_folders: bool) {
        if self
            .entries
            .get(idx)
            .is_some_and(|e| takes_group(e, select_folders))
        {
            self.mark(idx, selected);
        }
    }

    fn mark(&mut self, idx: usize, selected: bool) {
        let path = self.entries[idx].path.clone();
        if !selected {
            if self.selected_paths.remove(&path) {
                self.selection_order.retain(|p| p != &path);
            }
        } else if self.selected_paths.insert(path.clone()) {
            self.selection_order.push(path);
        }
    }

    /// Whether entry `idx` is tagged.
    pub fn is_selected(&self, idx: usize) -> bool {
        self.entries
            .get(idx)
            .is_some_and(|entry| self.selected_paths.contains(&entry.path))
    }

    /// Indexes of the entries a group selection matching `mask` takes.
    fn group(&self, mask: &str, select_folders: bool) -> Vec<usize> {
        (0..self.entries.len())
            .filter(|&i| {
                let entry = &self.entries[i];
                takes_group(entry, select_folders) && glob_matches(mask, &entry.name)
            })
            .collect()
    }

    /// Tags the entries matching a glob mask (Gray `+`, Select all).
    pub fn select_group(&mut self, mask: &str, select_folders: bool) {
        for idx in self.group(mask, select_folders) {
            self.mark(idx, true);
        }
    }

    /// Untags the entries matching a glob mask (Gray `-`).
    pub fn unselect_group(&mut self, mask: &str, select_folders: bool) {
        for idx in self.group(mask, select_folders) {
            self.mark(idx, false);
        }
    }

    /// Inverts the tags of the entries a group selection takes (Gray `*`);
    /// the others keep theirs.
    pub fn invert_selection(&mut self, select_folders: bool) {
        let flipped: HashSet<PathBuf> = self
            .entries
            .iter()
            .filter(|e| takes_group(e, select_folders))
            .map(|e| e.path.clone())
            .collect();
        let selected = std::mem::take(&mut self.selected_paths);
        self.selected_paths = selected.symmetric_difference(&flipped).cloned().collect();
        // Listing order, as the inverted entries have no tagging order.
        self.selection_order.clear();
        for entry in &self.entries {
            if self.selected_paths.contains(&entry.path) {
                self.selection_order.push(entry.path.clone());
            }
        }
    }

    /// Untags everything.
    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.selection_order.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::panel::PanelState;
    use crate::fs::FileEntry;
    use std::path::PathBuf;

    /// `/d` with `..`, folder `sub` and files `a.rs`, `b.rs`.
    fn panel() -> PanelState {
        let mut p = PanelState::new(PathBuf::from("/d"));
        p.entries = [
            ("..", true),
            ("sub", true),
            ("a.rs", false),
            ("b.rs", false),
        ]
        .iter()
        .map(|&(name, is_dir)| FileEntry {
            name: name.into(),
            path: PathBuf::from("/d").join(name),
            is_dir,
            ..FileEntry::default()
        })
        .collect();
        p
    }

    fn tagged(p: &PanelState) -> Vec<String> {
        let mut names: Vec<_> = p
            .selected_paths
            .iter()
            .map(|x| x.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_single_toggle_takes_a_folder_whatever_the_setting() {
        let mut p = panel();
        p.cursor_index = 1;
        p.toggle_selection();
        assert_eq!(tagged(&p), ["sub"]);
        p.cursor_index = 0;
        p.toggle_selection();
        assert_eq!(tagged(&p), ["sub"], "`..` never");
    }

    #[test]
    fn masks_and_invert_take_folders_only_with_the_setting() {
        let mut p = panel();
        p.select_group("*", false);
        assert_eq!(tagged(&p), ["a.rs", "b.rs"]);
        p.select_group("*", true);
        assert_eq!(tagged(&p), ["a.rs", "b.rs", "sub"]);
        p.unselect_group("*", false);
        assert_eq!(tagged(&p), ["sub"], "a folder tagged by hand stays");
        p.invert_selection(false);
        assert_eq!(tagged(&p), ["a.rs", "b.rs", "sub"]);
        p.invert_selection(true);
        assert!(tagged(&p).is_empty());
    }

    #[test]
    fn visual_mode_follows_the_group_rule() {
        for (select_folders, expected) in [
            (false, vec!["a.rs", "b.rs"]),
            (true, vec!["a.rs", "b.rs", "sub"]),
        ] {
            let mut p = panel();
            p.toggle_visual(select_folders);
            p.cursor_index = 3;
            p.extend_visual();
            assert_eq!(tagged(&p), expected, "select_folders {select_folders}");
        }
    }

    #[test]
    fn selection_order_follows_tagging() {
        let mut p = panel();
        p.cursor_index = 3;
        p.toggle_selection();
        p.cursor_index = 2;
        p.toggle_selection();
        assert_eq!(
            p.selection_order,
            [PathBuf::from("/d/b.rs"), PathBuf::from("/d/a.rs")]
        );
        p.cursor_index = 3;
        p.toggle_selection();
        assert_eq!(p.selection_order, [PathBuf::from("/d/a.rs")]);
        p.clear_selection();
        assert!(p.selection_order.is_empty() && p.selected_paths.is_empty());
    }
}
