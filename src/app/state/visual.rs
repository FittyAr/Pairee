//! Visual selection (Vim / yazi `v`): while active, the entries between the
//! anchor and the cursor are selected on top of what was selected before.

use super::panel::PanelState;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VisualSelection {
    anchor: usize,
    /// Selection when visual mode started, in selection order.
    base: Vec<PathBuf>,
}

impl PanelState {
    /// Starts visual selection at the cursor, or leaves it (keeping what is
    /// selected).
    pub fn toggle_visual(&mut self) {
        if self.visual.take().is_none() {
            self.visual = Some(VisualSelection {
                anchor: self.cursor_index,
                base: self.selection_order.clone(),
            });
            self.extend_visual();
        }
    }

    /// Re-applies the visual range after the cursor moved.
    pub fn extend_visual(&mut self) {
        let Some(visual) = &self.visual else {
            return;
        };
        let (lo, hi) = if visual.anchor <= self.cursor_index {
            (visual.anchor, self.cursor_index)
        } else {
            (self.cursor_index, visual.anchor)
        };
        let mut order = visual.base.clone();
        let range = self.entries.iter().skip(lo).take(hi + 1 - lo);
        for entry in range.filter(|e| e.name != "..") {
            if !order.contains(&entry.path) {
                order.push(entry.path.clone());
            }
        }
        self.selected_paths = order.iter().cloned().collect();
        self.selection_order = order;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::panel::PanelState;
    use crate::fs::FileEntry;
    use std::path::PathBuf;

    fn panel(names: &[&str]) -> PanelState {
        let mut p = PanelState::new(PathBuf::from("/d"));
        p.entries = names
            .iter()
            .map(|n| FileEntry {
                name: n.to_string(),
                path: PathBuf::from("/d").join(n),
                size: 0,
                is_dir: false,
                is_symlink: false,
                modified: None,
            })
            .collect();
        p
    }

    #[test]
    fn range_follows_the_cursor_both_ways_and_keeps_the_base() {
        let mut p = panel(&["..", "a", "b", "c", "d"]);
        p.selected_paths.insert(PathBuf::from("/d/d"));
        p.selection_order.push(PathBuf::from("/d/d"));
        p.cursor_index = 2;
        p.toggle_visual();
        p.cursor_index = 3;
        p.extend_visual();
        assert_eq!(p.selected_paths.len(), 3, "d + b..c");
        p.cursor_index = 0;
        p.extend_visual();
        let mut names: Vec<_> = p
            .selection_order
            .iter()
            .map(|x| x.file_name().unwrap().to_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a", "b", "d"], "`..` is never selected");
        p.toggle_visual();
        assert!(p.visual.is_none());
        assert_eq!(p.selected_paths.len(), 3, "leaving keeps the selection");
    }
}
