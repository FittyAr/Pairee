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

/// Initializes a new empty Git repository at the specified path.
pub fn init_repo(path: &Path) -> anyhow::Result<git2::Repository> {
    let repo = git2::Repository::init(path)?;
    Ok(repo)
}

/// Clones a remote repository to the specified path.
pub fn clone_repo(url: &str, path: &Path) -> anyhow::Result<git2::Repository> {
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(crate::git::remote::create_callbacks());

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let repo = builder.clone(url, path)?;
    Ok(repo)
}

/// Appends a pattern or file path to the repository's `.gitignore` file.
pub fn add_to_gitignore(repo: &git2::Repository, pattern: &str) -> anyhow::Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;
    let gitignore_path = workdir.join(".gitignore");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)?;
    writeln!(file, "{}", pattern)?;
    Ok(())
}
