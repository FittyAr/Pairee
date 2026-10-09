use crate::git::branches::*;
use crate::git::checkout::*;
use crate::git::cherry_pick::*;
use crate::git::commit::*;
use crate::git::diff::*;
use crate::git::log::*;
use crate::git::merge::*;
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

mod diff;
mod remote;
mod workflow;
