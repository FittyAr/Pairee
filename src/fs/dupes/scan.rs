//! First pass of the duplicate finder: every file below the roots, grouped
//! by size.

use super::group::Candidate;
use super::{DupeControl, DupeOptions, DupeProgress, DupeReport, DupeStage};
use crate::fs::vfs::Vfs;
use crate::fs::walk::{LinkPolicy, WalkError, WalkOptions, Walker};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Minimum time between two progress reports.
pub(super) const REPORT_EVERY: Duration = Duration::from_millis(100);

/// Files by size.
type BySize = BTreeMap<u64, Vec<PathBuf>>;

/// Files of `roots` grouped by size (only sizes two files or more share).
/// Links are left out, hard links of one file and paths reached from two
/// roots count once.
pub(super) fn files_by_size(
    vfs: &dyn Vfs,
    roots: &[PathBuf],
    options: &DupeOptions,
    ctl: &DupeControl,
    report: &mut DupeReport,
) -> Vec<Candidate> {
    let mut sizes = BySize::new();
    let mut seen_paths = HashSet::new();
    let mut seen_links = HashSet::new();
    let mut progress = DupeProgress {
        stage: DupeStage::Scanning,
        done: 0,
        total: 0,
        current: PathBuf::new(),
    };
    let mut last_report = Instant::now();
    for root in roots {
        let walker = Walker::new(vfs, root, WalkOptions::with_links(LinkPolicy::Skip))
            .with_cancel(ctl.cancelled);
        for item in walker {
            let found = match item {
                Ok(found) => found,
                Err(WalkError::Cancelled) => {
                    report.partial = true;
                    return keep_shared(sizes);
                }
                Err(e) => {
                    report.partial = true;
                    report.unreadable.extend(e.path().map(PathBuf::from));
                    continue;
                }
            };
            let entry = found.entry;
            if found.descends || entry.is_dir {
                continue;
            }
            if entry.unreadable {
                report.partial = true;
                report.unreadable.push(entry.path);
                continue;
            }
            let first_link = entry.hard_link.is_none_or(|id| seen_links.insert(id));
            if entry.size < options.min_size
                || !first_link
                || !seen_paths.insert(entry.path.clone())
            {
                continue;
            }
            progress.done += 1;
            if last_report.elapsed() >= REPORT_EVERY {
                last_report = Instant::now();
                progress.current = entry.path.clone();
                (ctl.progress)(&progress);
            }
            sizes.entry(entry.size).or_default().push(entry.path);
        }
    }
    keep_shared(sizes)
}

/// The sizes two files or more share, as candidate groups.
fn keep_shared(sizes: BySize) -> Vec<Candidate> {
    sizes
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(size, files)| Candidate {
            size,
            hash: None,
            files,
        })
        .collect()
}
