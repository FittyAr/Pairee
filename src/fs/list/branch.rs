//! Branch view listing (Total Commander's Ctrl+B): every file below a
//! folder in one flat list, each named by its path relative to the folder.
//!
//! The tree is read by the shared walker ([`Walker`]) over any [`Vfs`], so
//! local folders, SFTP servers and archives work alike. Links are listed but
//! never entered, hidden folders are not entered unless hidden files are
//! shown, and subfolders that cannot be read are left out (the listing is
//! still useful). Only an unreadable root fails the listing.

use super::ListOptions;
use crate::fs::FileEntry;
use crate::fs::vfs::{Vfs, VfsEntry, parent_entry};
use crate::fs::walk::{LinkPolicy, WalkError, WalkOptions, Walker};
use std::path::Path;
use std::time::{Duration, Instant};

/// Minimum time between two progress reports.
const REPORT_EVERY: Duration = Duration::from_millis(100);

/// Cancellation and progress hooks of [`read_branch`].
pub struct BranchControl<'a> {
    /// Polled before each entry.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Receives the number of files found so far (rate-limited).
    pub progress: &'a dyn Fn(u64),
}

/// Every file below `root` on `vfs`, named by its path relative to `root`,
/// after the `..` entry and sorted as `opts` say. A cancelled walk returns
/// what it found so far.
pub fn read_branch(
    vfs: &dyn Vfs,
    root: &Path,
    opts: &ListOptions,
    ctl: &BranchControl,
) -> anyhow::Result<Vec<FileEntry>> {
    // The root must be readable: the walker would only report it as an item.
    vfs.list(root)?;
    let mut entries: Vec<FileEntry> = parent_entry(root, opts.show_dotdot).into_iter().collect();
    let mut walker = Walker::new(vfs, root, WalkOptions::with_links(LinkPolicy::Keep))
        .with_cancel(ctl.cancelled);
    let mut last_report = Instant::now();
    while let Some(item) = walker.next() {
        let found = match item {
            Ok(found) => found,
            Err(WalkError::Cancelled) => break,
            Err(e) => {
                log::debug!("Branch view of {}: {e}", root.display());
                continue;
            }
        };
        let hidden = found.entry.hidden && !opts.show_hidden;
        if found.descends {
            if hidden {
                walker.skip_dir();
            }
            continue;
        }
        if hidden || found.entry.is_dir {
            continue;
        }
        entries.push(branch_entry(root, found.entry));
        if last_report.elapsed() >= REPORT_EVERY {
            last_report = Instant::now();
            (ctl.progress)(entries.len() as u64);
        }
    }
    opts.sort(&mut entries);
    Ok(entries)
}

/// `entry` named by its path relative to `root`.
fn branch_entry(root: &Path, entry: VfsEntry) -> FileEntry {
    let mut file = entry.into_file_entry();
    if let Ok(relative) = file.path.strip_prefix(root) {
        file.name = relative.to_string_lossy().into_owned();
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::list::SortField;
    use crate::fs::vfs::LocalVfs;
    use std::cell::Cell;

    fn options(show_hidden: bool) -> ListOptions {
        ListOptions {
            show_hidden,
            case_sensitive: false,
            natural: false,
            req_admin: false,
            sort_field: SortField::Name,
            sort_reverse: false,
            folder_by_ext: false,
            show_dotdot: true,
        }
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a").join("b")).unwrap();
        std::fs::create_dir_all(p.join(".git")).unwrap();
        std::fs::write(p.join("top.txt"), b"1").unwrap();
        std::fs::write(p.join("a").join("one.rs"), b"22").unwrap();
        std::fs::write(p.join("a").join("b").join("deep.txt"), b"333").unwrap();
        std::fs::write(p.join(".git").join("config"), b"x").unwrap();
        dir
    }

    fn names(dir: &Path, show_hidden: bool) -> Vec<String> {
        let ctl = BranchControl {
            cancelled: &|| false,
            progress: &|_| {},
        };
        read_branch(&LocalVfs, dir, &options(show_hidden), &ctl)
            .unwrap()
            .into_iter()
            .map(|e| e.name.replace('\\', "/"))
            .collect()
    }

    #[test]
    fn lists_every_file_flat_with_relative_names() {
        let dir = tree();
        assert_eq!(
            names(dir.path(), false),
            ["..", "a/b/deep.txt", "a/one.rs", "top.txt"]
        );
    }

    #[test]
    fn hidden_folders_are_entered_only_when_hidden_files_show() {
        let dir = tree();
        assert!(names(dir.path(), true).contains(&".git/config".to_string()));
        assert!(!names(dir.path(), false).contains(&".git/config".to_string()));
    }

    #[test]
    fn entries_keep_their_full_path_and_size() {
        let dir = tree();
        let ctl = BranchControl {
            cancelled: &|| false,
            progress: &|_| {},
        };
        let entries = read_branch(&LocalVfs, dir.path(), &options(false), &ctl).unwrap();
        let deep = entries
            .iter()
            .find(|e| e.name.ends_with("deep.txt"))
            .unwrap();
        assert_eq!(deep.path, dir.path().join("a").join("b").join("deep.txt"));
        assert_eq!(deep.size, 3);
        assert!(!deep.is_dir);
    }

    #[test]
    fn a_cancelled_walk_returns_what_it_found() {
        let dir = tree();
        let polls = Cell::new(0);
        let cancelled = || {
            polls.set(polls.get() + 1);
            polls.get() > 2
        };
        let ctl = BranchControl {
            cancelled: &cancelled,
            progress: &|_| {},
        };
        let entries = read_branch(&LocalVfs, dir.path(), &options(false), &ctl).unwrap();
        assert!(entries.len() < 4, "{entries:?}");
    }

    #[test]
    fn an_unreadable_root_fails() {
        let dir = tempfile::tempdir().unwrap();
        let ctl = BranchControl {
            cancelled: &|| false,
            progress: &|_| {},
        };
        let missing = dir.path().join("missing");
        assert!(read_branch(&LocalVfs, &missing, &options(false), &ctl).is_err());
    }
}
