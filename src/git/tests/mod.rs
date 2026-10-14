use crate::git::branches::*;
use crate::git::checkout::*;
use crate::git::cherry_pick::*;
use crate::git::commit::*;
use crate::git::diff::*;
use crate::git::log::*;
use crate::git::merge::*;
use crate::git::outcome::*;
use crate::git::rebase::*;
use crate::git::remote::*;
use crate::git::repo::*;
use crate::git::reset::*;
use crate::git::revert::*;
use crate::git::stage::*;
use crate::git::stash::*;
use crate::git::status::*;
use crate::git::tags::*;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn setup_temp_repo() -> (TempDir, git2::Repository) {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path()).unwrap();

    // Configure signature
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    (dir, repo)
}

/// Writes `content` to `name` in the repo's workdir, stages it and commits.
fn write_and_commit(repo: &git2::Repository, name: &str, content: &str, msg: &str) -> git2::Oid {
    write_file(repo, name, content);
    stage_file(repo, name).unwrap();
    commit(repo, msg, "Test User", "test@example.com").unwrap()
}

/// Overwrites `name` in the repo's workdir without staging it.
fn write_file(repo: &git2::Repository, name: &str, content: &str) {
    std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
}

/// Reads `name` from the repo's workdir.
fn read_file(repo: &git2::Repository, name: &str) -> String {
    std::fs::read_to_string(repo.workdir().unwrap().join(name)).unwrap()
}

/// The checked-out branch's short name.
fn current_branch(repo: &git2::Repository) -> String {
    repo.head().unwrap().shorthand().unwrap().to_string()
}

/// A bare repository usable as a local remote, plus its URL.
fn bare_remote() -> (TempDir, git2::Repository, String) {
    let dir = TempDir::new().unwrap();
    let url = dir.path().to_str().unwrap().replace('\\', "/");
    let repo = git2::Repository::init_bare(dir.path()).unwrap();
    (dir, repo, url)
}

mod credentials;
mod diff;
mod remote;
mod state;
mod sync;
mod workflow;
