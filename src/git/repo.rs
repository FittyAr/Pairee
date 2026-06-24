use std::path::Path;

/// Tries to find a git repository starting from `path` and walking up the directory tree.
/// Returns `Some(git2::Repository)` if found, `None` otherwise.
pub fn find_repo(path: &Path) -> Option<git2::Repository> {
    git2::Repository::discover(path).ok()
}

/// Returns the path to the root of the repository's working directory.
pub fn get_workdir(repo: &git2::Repository) -> Option<std::path::PathBuf> {
    repo.workdir().map(|p| p.to_path_buf())
}
