//! The recursive walker shared by folder sizes and the disk usage view.

use super::{DirSize, DuEntry, DuKind, DuNode, DuSource, ScanProgress};
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

/// Deeper trees are not descended (the result is marked partial). Links are
/// not followed, so only pathological bind mounts can get this deep.
const MAX_DEPTH: usize = 1024;

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
pub fn scan(source: &dyn DuSource, root: &Path, keep_tree: bool, ctl: &ScanControl) -> DuNode {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    let mut walker = Walker {
        source,
        ctl,
        keep_tree,
        seen: HashSet::new(),
        progress: ScanProgress::default(),
        last_report: Instant::now(),
    };
    let node = walker.dir(root, name, 0);
    (ctl.progress)(&walker.progress);
    node
}

struct Walker<'a> {
    source: &'a dyn DuSource,
    ctl: &'a ScanControl<'a>,
    keep_tree: bool,
    /// Hard-linked files already counted.
    seen: HashSet<(u64, u64)>,
    progress: ScanProgress,
    last_report: Instant,
}

impl Walker<'_> {
    fn dir(&mut self, path: &Path, name: String, depth: usize) -> DuNode {
        let mut node = DuNode::dir(name);
        if depth > MAX_DEPTH || (self.ctl.cancelled)() {
            node.size.partial = true;
            return node;
        }
        self.progress.current = path.to_path_buf();
        let Ok(entries) = self.source.read_dir(path) else {
            node.size.partial = true;
            return node;
        };
        for entry in entries {
            if (self.ctl.cancelled)() {
                node.size.partial = true;
                break;
            }
            // Unreadable entries only flag the totals as partial.
            let keep = self.keep_tree && entry.kind != DuKind::Unreadable;
            let child = match entry.kind {
                DuKind::Dir => {
                    self.progress.dirs += 1;
                    self.dir(&entry.path, entry.name, depth + 1)
                }
                DuKind::File | DuKind::Unreadable => self.file(entry),
            };
            node.attach(child, keep);
        }
        node.sort_children();
        node
    }

    fn file(&mut self, entry: DuEntry) -> DuNode {
        let unreadable = entry.kind == DuKind::Unreadable;
        // A hard link seen before adds nothing (but is still listed).
        let first_link = entry.id.is_none_or(|id| self.seen.insert(id));
        let size = DirSize {
            bytes: if first_link { entry.size } else { 0 },
            files: u64::from(!unreadable),
            dirs: 0,
            partial: unreadable,
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
