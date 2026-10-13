//! Folder sizes on the shared tree walker ([`crate::fs::walk::Walker`]):
//! links are kept (counted with their own size, never entered), every
//! problem the walker reports only marks the totals partial.

use super::{DirSize, DuNode, ScanProgress};
use crate::fs::vfs::{Vfs, VfsEntry};
use crate::fs::walk::{LinkPolicy, WalkEntry, WalkError, WalkOptions, Walker};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Minimum time between two progress reports.
const REPORT_EVERY: Duration = Duration::from_millis(50);

/// Cancellation and progress hooks for [`scan`].
pub struct ScanControl<'a> {
    /// Polled between entries; when it returns `true` the scan stops and the
    /// result is marked partial.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Receives running totals (rate-limited).
    pub progress: &'a dyn Fn(&ScanProgress),
}

/// Scans the directory `root`. With `keep_tree` the returned node holds the
/// whole tree (disk usage view); otherwise only its totals (folder size).
pub fn scan(source: &dyn Vfs, root: &Path, keep_tree: bool, ctl: &ScanControl) -> DuNode {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    let mut sizes = Sizes {
        tree: Tree::new(root.to_path_buf(), name, keep_tree),
        ctl,
        seen: HashSet::new(),
        progress: ScanProgress {
            current: root.to_path_buf(),
            ..ScanProgress::default()
        },
        last_report: Instant::now(),
    };
    let walker = Walker::new(source, root, WalkOptions::with_links(LinkPolicy::Keep))
        .with_cancel(ctl.cancelled);
    for item in walker {
        match item {
            Ok(found) => sizes.found(found),
            Err(WalkError::Cancelled) => {
                sizes.tree.mark_open_partial();
                break;
            }
            // Unreadable or too deep folders only flag the totals.
            Err(e) => sizes.tree.mark_partial(e.path().unwrap_or(root)),
        }
    }
    (ctl.progress)(&sizes.progress);
    sizes.tree.finish()
}

/// The running scan: the tree being built and the progress totals.
struct Sizes<'a> {
    tree: Tree,
    ctl: &'a ScanControl<'a>,
    /// Hard-linked files already counted.
    seen: HashSet<(u64, u64)>,
    progress: ScanProgress,
    last_report: Instant,
}

impl Sizes<'_> {
    fn found(&mut self, found: WalkEntry) {
        self.tree.close_to(found.depth);
        if found.descends {
            self.progress.dirs += 1;
            self.progress.current = found.entry.path.clone();
            self.tree.open(found.entry);
            return;
        }
        let file = self.file(found.entry);
        self.tree.leaf(file);
    }

    fn file(&mut self, entry: VfsEntry) -> DuNode {
        // A hard link seen before adds nothing (but is still listed).
        let first_link = entry.hard_link.is_none_or(|id| self.seen.insert(id));
        let size = DirSize {
            bytes: if first_link && !entry.unreadable {
                entry.own_size()
            } else {
                0
            },
            files: u64::from(!entry.unreadable),
            dirs: 0,
            partial: entry.unreadable,
        };
        self.progress.bytes = self.progress.bytes.saturating_add(size.bytes);
        self.progress.files += size.files;
        self.maybe_report();
        DuNode {
            name: entry.name,
            is_dir: false,
            size,
            children: Vec::new(),
        }
    }

    fn maybe_report(&mut self) {
        if self.last_report.elapsed() >= REPORT_EVERY {
            self.last_report = Instant::now();
            (self.ctl.progress)(&self.progress);
        }
    }
}

/// The folders from the root to the one being walked (`open[d]` is at
/// depth `d`), each with what was attached to it so far.
struct Tree {
    open: Vec<(PathBuf, DuNode)>,
    keep: bool,
}

impl Tree {
    fn new(root: PathBuf, name: String, keep: bool) -> Self {
        Self {
            open: vec![(root, DuNode::dir(name))],
            keep,
        }
    }

    /// Closes the folders deeper than the parent of an entry at `depth`.
    fn close_to(&mut self, depth: usize) {
        while self.open.len() > depth.max(1) {
            self.close_last();
        }
    }

    fn close_last(&mut self) {
        let Some((_, mut done)) = self.open.pop() else {
            return;
        };
        done.sort_children();
        let keep = self.keep;
        if let Some((_, parent)) = self.open.last_mut() {
            parent.attach(done, keep);
        }
    }

    fn open(&mut self, entry: VfsEntry) {
        self.open.push((entry.path, DuNode::dir(entry.name)));
    }

    /// Attaches a file to the folder being walked (unreadable entries only
    /// count towards the partial flag).
    fn leaf(&mut self, node: DuNode) {
        let keep = self.keep && !node.size.partial;
        if let Some((_, parent)) = self.open.last_mut() {
            parent.attach(node, keep);
        }
    }

    /// Flags the folder `path` (or the deepest open folder holding it).
    fn mark_partial(&mut self, path: &Path) {
        while self.open.len() > 1 && !self.open.last().is_some_and(|(p, _)| path.starts_with(p)) {
            self.close_last();
        }
        if let Some((_, node)) = self.open.last_mut() {
            node.size.partial = true;
        }
    }

    /// Flags every open folder (a cancelled scan).
    fn mark_open_partial(&mut self) {
        for (_, node) in &mut self.open {
            node.size.partial = true;
        }
    }

    fn finish(mut self) -> DuNode {
        self.close_to(1);
        let (_, mut root) = self.open.pop().unwrap_or_default();
        root.sort_children();
        root
    }
}
