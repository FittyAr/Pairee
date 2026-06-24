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
