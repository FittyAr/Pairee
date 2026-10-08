//! In-memory size tree built by a disk usage scan.

use super::DirSize;

/// A scanned file or directory. Directory children are sorted by size,
/// largest first; they are only kept when the scan asked for a tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DuNode {
    pub name: String,
    pub is_dir: bool,
    /// Totals of everything below (for a file: its own size, `files == 1`).
    pub size: DirSize,
    pub children: Vec<DuNode>,
}

impl DuNode {
    pub(super) fn dir(name: String) -> Self {
        Self {
            name,
            is_dir: true,
            ..Self::default()
        }
    }

    /// What this node adds to its parent's totals (a directory also counts
    /// itself as one subdirectory).
    pub fn contribution(&self) -> DirSize {
        let mut size = self.size;
        if self.is_dir {
            size.dirs = size.dirs.saturating_add(1);
        }
        size
    }

    /// Adds `child` to the totals and, when `keep` is set, to the children.
    pub(super) fn attach(&mut self, child: DuNode, keep: bool) {
        self.size.add(&child.contribution());
        if keep {
            self.children.push(child);
        }
    }

    /// Largest children first, ties broken by name.
    pub(super) fn sort_children(&mut self) {
        self.children.sort_by(|a, b| {
            b.size
                .bytes
                .cmp(&a.size.bytes)
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    /// The node reached by following child `names` from here.
    pub fn descend<S: AsRef<str>>(&self, names: &[S]) -> Option<&DuNode> {
        names.iter().try_fold(self, |node, name| {
            node.children.iter().find(|c| c.name == name.as_ref())
        })
    }

    /// Removes the node at `names` (relative to here) and subtracts it from
    /// every ancestor. Returns what was subtracted.
    pub fn remove<S: AsRef<str>>(&mut self, names: &[S]) -> Option<DirSize> {
        let (first, rest) = names.split_first()?;
        let idx = self
            .children
            .iter()
            .position(|c| c.name == first.as_ref())?;
        let removed = if rest.is_empty() {
            self.children.remove(idx).contribution()
        } else {
            self.children[idx].remove(rest)?
        };
        self.size.sub(&removed);
        Some(removed)
    }

    /// Share of `total` taken by this node, in percent (0 when `total` is 0).
    pub fn percent_of(&self, total: u64) -> f64 {
        if total == 0 {
            0.0
        } else {
            self.size.bytes as f64 * 100.0 / total as f64
        }
    }
}
