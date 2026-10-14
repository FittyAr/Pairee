//! Git side of a release: a clean tree, the release branch, a free tag, and
//! one atomic push of the branch and the tag.

use std::process::Command;

use super::winget;
use crate::{Result, cmd, ui};

/// Files the release rewrites and commits; local edits to them are allowed
/// (they become part of the release commit).
fn is_managed(path: &str) -> bool {
    super::RELEASE_FILES.contains(&path)
        || path
            .strip_prefix(winget::WINGET_DIR)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Paths with uncommitted changes, untracked files included.
fn dirty_paths() -> Result<Vec<String>> {
    let out = cmd::raw_output(
        "git",
        &["status", "--porcelain", "-z", "--untracked-files=all"],
    )?;
    Ok(parse_status(&out))
}

/// Parses `git status --porcelain -z`: `XY path`, followed by the original
/// path for a rename or copy.
fn parse_status(out: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut entries = out.split('\0').filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        let Some((xy, path)) = entry.split_at_checked(3) else {
            continue;
        };
        paths.push(path.to_string());
        if xy.starts_with(['R', 'C'])
            && let Some(original) = entries.next()
        {
            paths.push(original.to_string());
        }
    }
    paths
}

/// Refuses a tree with changes outside the files the release manages: the
/// local gate would test code that the release commit does not contain.
pub fn ensure_clean_tree() -> Result {
    let foreign: Vec<String> = dirty_paths()?
        .into_iter()
        .filter(|p| !is_managed(p))
        .collect();
    if foreign.is_empty() {
        return Ok(());
    }
    Err(format!(
        "uncommitted changes outside the release files; commit or stash them first:\n  {}",
        foreign.join("\n  ")
    )
    .into())
}

fn branch_exists(name: &str) -> bool {
    cmd::quietly(Command::new("git").args([
        "show-ref",
        "--verify",
        "--quiet",
        &format!("refs/heads/{name}"),
    ]))
}

/// Releases are cut from the main branch: offers to merge a feature branch
/// into it first. Returns the branch to release from.
pub fn prepare(yes: bool) -> Result<String> {
    let main = if branch_exists("master") && !branch_exists("main") {
        "master"
    } else {
        "main"
    };
    let branch = cmd::output("git", &["branch", "--show-current"])?;
    if branch.is_empty() {
        return Err("HEAD is detached; check out a branch first".into());
    }
    if branch == main {
        return Ok(branch);
    }

    ui::warn(&format!(
        "you are on '{branch}', not on the release branch '{main}'"
    ));
    if ui::confirm(
        &format!("Merge '{branch}' into '{main}' and release from '{main}'?"),
        yes,
    )? {
        merge_into(&branch, main)?;
        return Ok(main.to_string());
    }
    if ui::confirm(&format!("Release directly from '{branch}'?"), false)? {
        Ok(branch)
    } else {
        Err("aborted".into())
    }
}

/// Fast-forwards `main` from origin and merges `branch` into it. Any
/// failure puts the repository back on `branch`, as it was.
fn merge_into(branch: &str, main: &str) -> Result {
    if !dirty_paths()?.is_empty() {
        return Err(format!("commit or stash the changes in '{branch}' first").into());
    }
    cmd::run("git", &["checkout", main])?;
    let back = |reason: String| -> Result {
        let _ = cmd::run("git", &["checkout", branch]);
        Err(reason.into())
    };
    if cmd::run("git", &["pull", "--ff-only", "origin", main]).is_err() {
        return back(format!(
            "'{main}' cannot be fast-forwarded from origin (it has local commits or diverged); \
             reconcile it and re-run"
        ));
    }
    if cmd::run("git", &["merge", "--no-edit", branch]).is_err() {
        let _ = cmd::run("git", &["merge", "--abort"]);
        return back(format!(
            "merging '{branch}' into '{main}' failed; the merge was aborted. \
             Resolve it by hand and re-run"
        ));
    }
    Ok(())
}

pub fn ensure_push_access(branch: &str) -> Result {
    ui::step(&format!("Checking push access to origin/{branch}"));
    let ok = cmd::quietly(
        Command::new("git")
            .args(["push", "--dry-run", "origin", branch])
            .env("GIT_TERMINAL_PROMPT", "0"),
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "cannot push to origin (log in with `gh auth login` or your credential helper; \
             test with `git push --dry-run origin {branch}`)"
        )
        .into())
    }
}

/// Fails when `tag` exists locally or, with `check_remote`, on origin.
pub fn ensure_tag_is_free(tag: &str, check_remote: bool) -> Result {
    if !cmd::output("git", &["tag", "--list", tag])?.is_empty() {
        return Err(format!("tag {tag} already exists").into());
    }
    if check_remote {
        let remote = cmd::output(
            "git",
            &["ls-remote", "--tags", "origin", &format!("refs/tags/{tag}")],
        )
        .map_err(|e| format!("cannot list the tags on origin: {e}"))?;
        if !remote.is_empty() {
            return Err(format!("tag {tag} already exists on origin").into());
        }
    }
    Ok(())
}

pub fn commit_and_tag(files: &[String], tag: &str) -> Result {
    let mut add = vec!["add", "--all", "--"];
    add.extend(files.iter().map(String::as_str));
    cmd::run("git", &add)?;
    cmd::run("git", &["commit", "-m", &format!("Bump version to {tag}")])?;
    cmd::run("git", &["tag", "-a", tag, "-m", &format!("Release {tag}")])
}

/// Pushes the branch and the tag in one atomic push: either both reach
/// origin or neither does.
pub fn push(branch: &str, tag: &str) -> Result {
    cmd::run("git", &["push", "--atomic", "origin", branch, tag]).map_err(|e| {
        format!(
            "{e}\nNothing was pushed; the release commit and {tag} exist locally. \
             Fix the cause and run: git push --atomic origin {branch} {tag}"
        )
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_paths_include_renames_and_untracked() {
        let out = " M Cargo.toml\0R  new.rs\0old.rs\0?? src/x y.rs\0";
        assert_eq!(
            parse_status(out),
            ["Cargo.toml", "new.rs", "old.rs", "src/x y.rs"]
        );
    }

    #[test]
    fn release_files_are_managed() {
        assert!(is_managed("Cargo.lock"));
        assert!(is_managed("docs/UNRELEASED.md"));
        assert!(is_managed(
            "manifests/f/FittyAr/Pairee/0.10.0/FittyAr.Pairee.yaml"
        ));
        assert!(!is_managed("manifests/f/FittyAr/PaireeX/a.yaml"));
        assert!(!is_managed("src/main.rs"));
    }
}
