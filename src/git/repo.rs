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
///
/// Supports both SSH and HTTPS remotes. For SSH, the agent is tried
/// first, then the standard `~/.ssh/id_ed25519` / `id_ecdsa` /
/// `id_rsa` / `id_dsa` keys. For HTTPS, the user/pass helper or
/// the OS credential store is tried; if neither has credentials
/// the clone is attempted anonymously, which works for public
/// repositories.
pub fn clone_repo(url: &str, path: &Path) -> anyhow::Result<git2::Repository> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
            if let Some(proj_dir) = directories::BaseDirs::new() {
                let ssh_dir = proj_dir.home_dir().join(".ssh");
                for filename in crate::git::remote::SSH_KEY_FILENAMES {
                    let key_path = ssh_dir.join(filename);
                    if key_path.exists() {
                        if let Ok(cred) = git2::Cred::ssh_key(username, None, &key_path, None) {
                            return Ok(cred);
                        }
                    }
                }
            }
            return Err(git2::Error::from_str(
                "no usable SSH credentials: agent not running and no \
                 standard key files in ~/.ssh/",
            ));
        }

        // HTTPS path. We delegate to `Cred::default`, which uses
        // whatever credential helper the user has configured in
        // their git config (`credential.helper = store` / `cache`
        // / `osxkeychain` / `libsecret`). For public repositories
        // no credentials are needed and `default` returns
        // anonymous access. The previous code only configured
        // SSH callbacks, so any HTTPS clone (including the very
        // common `https://github.com/...` form) failed with a
        // useless "Authentication failed" error.
        return git2::Cred::default();
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let repo = builder.clone(url, path)?;
    Ok(repo)
}
