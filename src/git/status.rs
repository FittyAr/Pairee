/// Represents the git status of a single file in the working tree.
#[derive(Debug, Clone)]
pub struct GitFileStatus {
    /// Path relative to the repository root
    pub path: String,
    /// The kind of change
    pub kind: StatusKind,
}

/// The type of change a file has in the working tree / index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusKind {
    /// File modified in working tree or index
    Modified,
    /// File newly added (staged or untracked)
    Added,
    /// File deleted
    Deleted,
    /// File not tracked by git
    Untracked,
    /// File renamed
    Renamed,
    /// File has merge conflict
    Conflicted,
}

impl StatusKind {
    /// Returns a short single-character label for display.
    pub fn label(&self) -> &'static str {
        match self {
            StatusKind::Modified => "M",
            StatusKind::Added => "A",
            StatusKind::Deleted => "D",
            StatusKind::Untracked => "?",
            StatusKind::Renamed => "R",
            StatusKind::Conflicted => "!",
        }
    }
}

/// Reads all changed, staged and untracked files from the repository.
pub fn get_status(repo: &git2::Repository) -> Vec<GitFileStatus> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path().unwrap_or("").to_string();
            if path.is_empty() {
                return None;
            }

            let flags = entry.status();

            let kind = if flags.contains(git2::Status::CONFLICTED) {
                StatusKind::Conflicted
            } else if flags.contains(git2::Status::INDEX_NEW)
                || flags.contains(git2::Status::WT_NEW)
            {
                // Distinguish truly untracked from newly staged
                if flags.contains(git2::Status::WT_NEW) && !flags.contains(git2::Status::INDEX_NEW)
                {
                    StatusKind::Untracked
                } else {
                    StatusKind::Added
                }
            } else if flags.contains(git2::Status::INDEX_DELETED)
                || flags.contains(git2::Status::WT_DELETED)
            {
                StatusKind::Deleted
            } else if flags.contains(git2::Status::INDEX_RENAMED)
                || flags.contains(git2::Status::WT_RENAMED)
            {
                StatusKind::Renamed
            } else {
                StatusKind::Modified
            };

            Some(GitFileStatus { path, kind })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::commit::commit;
    use crate::git::repo::init_repo;
    use crate::git::stage::stage_file;
    use std::fs::File;
    use tempfile::TempDir;

    fn setup() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        let mut c = repo.config().unwrap();
        c.set_str("user.name", "T").unwrap();
        c.set_str("user.email", "t@t").unwrap();
        (dir, repo)
    }

    #[test]
    fn status_kind_label_is_single_char() {
        // Each variant must map to a single character for the
        // panel display. Pin the exact chars so a rename shows up
        // as a deliberate test change.
        assert_eq!(StatusKind::Modified.label(), "M");
        assert_eq!(StatusKind::Added.label(), "A");
        assert_eq!(StatusKind::Deleted.label(), "D");
        assert_eq!(StatusKind::Untracked.label(), "?");
        assert_eq!(StatusKind::Renamed.label(), "R");
        assert_eq!(StatusKind::Conflicted.label(), "!");
    }

    #[test]
    fn status_kind_equality_works() {
        assert_eq!(StatusKind::Modified, StatusKind::Modified);
        assert_ne!(StatusKind::Modified, StatusKind::Added);
    }

    #[test]
    fn get_status_on_clean_repo_is_empty() {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        assert!(get_status(&repo).is_empty());
    }

    #[test]
    fn get_status_picks_up_untracked_files() {
        let (dir, repo) = setup();
        // Add an untracked file (no stage, no commit).
        File::create(dir.path().join("new.txt")).unwrap();
        let statuses = get_status(&repo);
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].kind, StatusKind::Untracked);
        assert_eq!(statuses[0].path, "new.txt");
    }

    #[test]
    fn get_status_picks_up_modified_after_commit() {
        let (dir, repo) = setup();
        // Initial commit of a.txt.
        File::create(dir.path().join("a.txt")).unwrap();
        stage_file(&repo, "a.txt").unwrap();
        commit(&repo, "init", "T", "t@t").unwrap();
        // Modify a.txt in the working tree (not staged).
        std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
        let statuses = get_status(&repo);
        let modified: Vec<_> = statuses.iter().filter(|s| s.path == "a.txt").collect();
        assert_eq!(modified.len(), 1);
        // Working-tree modifications are reported as `Modified` by
        // the current mapping.
        assert_eq!(modified[0].kind, StatusKind::Modified);
    }
}
