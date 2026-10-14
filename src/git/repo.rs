use std::path::Path;

/// Tries to find a git repository starting from `path` and walking up the directory tree.
/// Returns `Some(git2::Repository)` if found, `None` otherwise.
pub fn find_repo(path: &Path) -> Option<git2::Repository> {
    git2::Repository::discover(path).ok()
}

/// The working directory, or a localized error for a bare repository.
pub fn workdir(repo: &git2::Repository) -> anyhow::Result<&Path> {
    repo.workdir()
        .ok_or_else(|| anyhow::anyhow!(crate::config::localization::t("git_error_no_workdir")))
}

/// Initializes a new empty Git repository at the specified path.
pub fn init_repo(path: &Path) -> anyhow::Result<git2::Repository> {
    let repo = git2::Repository::init(path)?;
    Ok(repo)
}

/// Clones a remote repository to the specified path.
pub fn clone_repo(
    url: &str,
    path: &Path,
    hooks: crate::git::remote::NetHooks<'_>,
) -> anyhow::Result<git2::Repository> {
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(crate::git::remote::create_callbacks(hooks));

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let repo = builder.clone(url, path)?;
    Ok(repo)
}

/// Appends a pattern or file path to the repository's `.gitignore` file,
/// on a line of its own even when the file does not end with a newline.
pub fn add_to_gitignore(repo: &git2::Repository, pattern: &str) -> anyhow::Result<()> {
    let workdir = workdir(repo)?;
    let gitignore_path = workdir.join(".gitignore");
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&gitignore_path)?;
    let mut last = [0u8; 1];
    let needs_newline = file.seek(SeekFrom::End(0))? > 0 && {
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut last)?;
        last[0] != b'\n'
    };
    let separator = if needs_newline { "\n" } else { "" };
    writeln!(file, "{separator}{pattern}")?;
    Ok(())
}
