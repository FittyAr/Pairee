//! Choosing what to delete among duplicates: which copy of a group stays
//! ([`Keep`]), and dropping deleted files from a report.

use super::DupeReport;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The copy of each group that stays unmarked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// The first by path.
    First,
    /// The most recently modified.
    Newest,
    /// The least recently modified.
    Oldest,
}

/// The files of a group to mark so that only the copy `keep` picks stays.
/// `modified` gives a file's modification time; files without one count
/// as the oldest. Ties keep the first by path.
pub fn all_but(
    files: &[PathBuf],
    keep: Keep,
    modified: impl Fn(&Path) -> Option<SystemTime>,
) -> Vec<PathBuf> {
    let kept = match keep {
        Keep::First => files.first(),
        // `max_by_key` returns the last of equal elements: walk backwards so
        // ties keep the first (`min_by_key` returns the first already).
        Keep::Newest => files.iter().rev().max_by_key(|f| modified(f)),
        Keep::Oldest => files.iter().min_by_key(|f| modified(f)),
    };
    files.iter().filter(|f| Some(*f) != kept).cloned().collect()
}

impl DupeReport {
    /// Drops `deleted` files from the groups; groups left with one file
    /// are no longer duplicates. Returns `true` if anything changed.
    pub fn forget<'a>(&mut self, deleted: impl IntoIterator<Item = &'a Path>) -> bool {
        let deleted: std::collections::HashSet<&Path> = deleted.into_iter().collect();
        let before: usize = self.groups.iter().map(|g| g.files.len()).sum();
        for group in &mut self.groups {
            group.files.retain(|f| !deleted.contains(f.as_path()));
        }
        self.groups.retain(|g| g.files.len() > 1);
        let after: usize = self.groups.iter().map(|g| g.files.len()).sum();
        before != after
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dupes::DupeGroup;
    use std::time::Duration;

    fn files() -> Vec<PathBuf> {
        ["/a", "/b", "/c"].map(PathBuf::from).to_vec()
    }

    fn at(secs: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
    }

    #[test]
    fn keeps_the_first_newest_or_oldest_copy() {
        let times = |p: &Path| match p.to_str() {
            Some("/a") => at(20),
            Some("/b") => at(30),
            _ => at(10),
        };
        let marked = |keep| all_but(&files(), keep, times);
        assert_eq!(marked(Keep::First), ["/b", "/c"].map(PathBuf::from));
        assert_eq!(marked(Keep::Newest), ["/a", "/c"].map(PathBuf::from));
        assert_eq!(marked(Keep::Oldest), ["/a", "/b"].map(PathBuf::from));
    }

    #[test]
    fn ties_and_unknown_times_keep_the_first() {
        let same = |_: &Path| at(5);
        assert_eq!(
            all_but(&files(), Keep::Newest, same),
            ["/b", "/c"].map(PathBuf::from)
        );
        let unknown = |p: &Path| (p != Path::new("/c")).then(|| at(5)).flatten();
        assert_eq!(
            all_but(&files(), Keep::Oldest, unknown),
            ["/a", "/b"].map(PathBuf::from)
        );
    }

    #[test]
    fn forgetting_files_drops_groups_left_alone() {
        let mut report = DupeReport {
            groups: vec![
                DupeGroup {
                    size: 1,
                    hash: "h".into(),
                    files: files(),
                },
                DupeGroup {
                    size: 2,
                    hash: "i".into(),
                    files: ["/x", "/y"].map(PathBuf::from).to_vec(),
                },
            ],
            ..Default::default()
        };
        assert!(report.forget([Path::new("/a"), Path::new("/x")]));
        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].files, ["/b", "/c"].map(PathBuf::from));
        assert!(!report.forget([Path::new("/zzz")]));
    }
}
