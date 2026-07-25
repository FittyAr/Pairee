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
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
        }
        Err(git2::Error::from_str("Authentication failed or no credentials found"))
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let repo = builder.clone(url, path)?;
    Ok(repo)
}
