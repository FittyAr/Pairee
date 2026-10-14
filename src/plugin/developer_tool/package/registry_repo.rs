//! The local clone of the plugin registry (`plugin-registry` branch) that
//! packaging writes into and submitting pushes from.
//!
//! The clone is kept between runs. Packaging a plugin stages (and commits)
//! it there until the user submits; refreshing the clone must never throw
//! that work away, offline or not.

use super::super::find_pairee_repo;

const BRANCH: &str = "plugin-registry";
const ORIGIN_REF: &str = "refs/remotes/origin/plugin-registry";
const GITHUB_URL: &str = "https://github.com/FittyAr/Pairee.git";

/// Updates the clone in `temp_dir` to the registry's latest state, or clones
/// the registry there. A clone holding work that was not pushed yet is
/// returned as it is (it is not reset to the registry, and it is not
/// deleted even when the registry cannot be reached).
pub fn fetch_or_clone_registry(temp_dir: &std::path::Path) -> anyhow::Result<git2::Repository> {
    if temp_dir.join(".git").exists() {
        if let Some(repo) = refresh_existing(temp_dir) {
            return Ok(repo);
        }
        // Nothing in it is worth keeping and it cannot be brought up to
        // date: start over.
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    std::fs::create_dir_all(temp_dir)?;

    let url = if let Some(local_path) = find_pairee_repo(BRANCH) {
        log::debug!(
            "plugin-registry: Using local repository clone for registry: {:?}",
            local_path
        );
        local_path.to_string_lossy().into_owned()
    } else {
        log::debug!(
            "plugin-registry: Local repo not found. Cloning registry from remote GitHub..."
        );
        GITHUB_URL.to_string()
    };

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch(BRANCH);
    let repo = builder.clone(&url, temp_dir)?;
    Ok(repo)
}

/// The existing clone, brought up to date when that loses nothing. `None`
/// when it is clean and cannot be updated (so it may be replaced).
fn refresh_existing(temp_dir: &std::path::Path) -> Option<git2::Repository> {
    let repo = git2::Repository::open(temp_dir).ok()?;
    let fetched = fetch_origin(&repo);
    if has_unpushed_work(&repo) {
        if !fetched {
            log::warn!("plugin-registry: offline; keeping the clone with its unsubmitted work");
        }
        return Some(repo);
    }
    reset_to_origin(&repo).then_some(repo)
}

fn fetch_origin(repo: &git2::Repository) -> bool {
    let Ok(mut remote) = repo.find_remote("origin") else {
        return false;
    };
    let mut fetch_options = git2::FetchOptions::new();
    remote
        .fetch(
            &[&format!("+refs/heads/{BRANCH}:{ORIGIN_REF}")],
            Some(&mut fetch_options),
            None,
        )
        .is_ok()
}

/// `true` when the clone has uncommitted changes, or commits the registry
/// does not have (or it cannot be told, which counts as work to keep).
fn has_unpushed_work(repo: &git2::Repository) -> bool {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true);
    let dirty = repo
        .statuses(Some(&mut options))
        .map(|statuses| !statuses.is_empty())
        .unwrap_or(true);
    if dirty {
        return true;
    }
    let head = repo.head().ok().and_then(|head| head.target());
    let upstream = repo.refname_to_id(ORIGIN_REF).ok();
    match (head, upstream) {
        (Some(head), Some(upstream)) => repo
            .graph_ahead_behind(head, upstream)
            .map(|(ahead, _)| ahead > 0)
            .unwrap_or(true),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// Checks out the registry's latest commit (as last fetched).
fn reset_to_origin(repo: &git2::Repository) -> bool {
    let Ok(commit) = repo
        .find_reference(ORIGIN_REF)
        .and_then(|reference| reference.peel_to_commit())
    else {
        return false;
    };
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    let _ = repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder));
    let _ = repo.set_head(&format!("refs/heads/{BRANCH}"));
    repo.reset(commit.as_object(), git2::ResetType::Hard, None)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A repository with one commit on `plugin-registry`.
    fn origin(dir: &std::path::Path) -> git2::Repository {
        let repo = git2::Repository::init(dir).unwrap();
        repo.set_head("refs/heads/plugin-registry").unwrap();
        commit_file(&repo, "index.toml", "registry", "first");
        repo
    }

    fn commit_file(repo: &git2::Repository, name: &str, body: &str, message: &str) {
        std::fs::write(repo.workdir().unwrap().join(name), body).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(name)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@example.com").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap();
    }

    fn clone_of(source: &git2::Repository, dir: &std::path::Path) -> git2::Repository {
        let mut builder = git2::build::RepoBuilder::new();
        builder.branch(BRANCH);
        builder
            .clone(source.workdir().unwrap().to_str().unwrap(), dir)
            .unwrap()
    }

    #[test]
    fn unpushed_work_survives_an_offline_refresh() {
        let root = tempfile::tempdir().unwrap();
        let upstream = origin(&root.path().join("origin"));
        let work = root.path().join("clone");
        let clone = clone_of(&upstream, &work);
        commit_file(&clone, "plugin.txt", "staged", "my plugin");
        // The registry is unreachable.
        clone
            .remote_set_url("origin", "/nonexistent/registry")
            .unwrap();
        drop(clone);

        let kept = refresh_existing(&work).expect("the clone is kept");
        assert!(kept.workdir().unwrap().join("plugin.txt").exists());
        // Uncommitted files are work too.
        std::fs::write(work.join("draft.txt"), "x").unwrap();
        assert!(has_unpushed_work(&kept));
    }

    #[test]
    fn a_clean_clone_follows_the_registry() {
        let root = tempfile::tempdir().unwrap();
        let upstream = origin(&root.path().join("origin"));
        let work = root.path().join("clone");
        drop(clone_of(&upstream, &work));
        commit_file(&upstream, "new.toml", "later", "second");
        let refreshed = refresh_existing(&work).expect("updated");
        assert!(refreshed.workdir().unwrap().join("new.toml").exists());
        assert!(!has_unpushed_work(&refreshed));
    }
}
