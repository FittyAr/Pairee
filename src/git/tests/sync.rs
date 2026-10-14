//! Fetch / pull / push against a local bare remote.

use super::*;
use crate::config::localization::t;

/// `local` (pushed, tracking `origin`) and `other` (a clone of the same
/// remote) sharing one bare remote. Both start at a commit with
/// `shared.txt` and `local.txt`.
struct Pair {
    _local_dir: TempDir,
    local: git2::Repository,
    remote_dir: TempDir,
    _other_dir: TempDir,
    other: git2::Repository,
}

fn setup_pair() -> Pair {
    let (local_dir, local) = setup_temp_repo();
    write_file(&local, "local.txt", "local v1");
    stage_file(&local, "local.txt").unwrap();
    write_and_commit(&local, "shared.txt", "shared v1", "initial");
    let (remote_dir, _bare, url) = bare_remote();
    add_remote(&local, "origin", &url).unwrap();
    push(
        &local,
        "origin",
        &current_branch(&local),
        true,
        NetHooks::default(),
    )
    .unwrap();

    let other_dir = TempDir::new().unwrap();
    let other = clone_repo(&url, &other_dir.path().join("other"), NetHooks::default()).unwrap();
    let mut config = other.config().unwrap();
    config.set_str("user.name", "Other User").unwrap();
    config.set_str("user.email", "other@example.com").unwrap();
    Pair {
        _local_dir: local_dir,
        local,
        remote_dir,
        _other_dir: other_dir,
        other,
    }
}

/// Commits `content` to `name` in `other` and pushes it.
fn other_pushes(pair: &Pair, name: &str, content: &str) -> git2::Oid {
    let oid = write_and_commit(&pair.other, name, content, "from other");
    push(
        &pair.other,
        "origin",
        &current_branch(&pair.other),
        false,
        NetHooks::default(),
    )
    .unwrap();
    oid
}

fn head_oid(repo: &git2::Repository) -> git2::Oid {
    repo.head().unwrap().target().unwrap()
}

#[test]
fn test_fetch_updates_remote_tracking_branch() {
    let pair = setup_pair();
    let pushed = other_pushes(&pair, "shared.txt", "shared v2");
    let branch = current_branch(&pair.local);

    fetch(&pair.local, "origin", NetHooks::default()).unwrap();

    let tracking = pair
        .local
        .find_reference(&format!("refs/remotes/origin/{branch}"))
        .unwrap();
    assert_eq!(tracking.target(), Some(pushed));
    // Fetch never touches the checked-out branch.
    assert_ne!(head_oid(&pair.local), pushed);
}

#[test]
fn test_pull_fast_forward_keeps_unrelated_local_changes() {
    let pair = setup_pair();
    let pushed = other_pushes(&pair, "shared.txt", "shared v2");
    write_file(&pair.local, "local.txt", "local edit");
    let branch = current_branch(&pair.local);

    pull(&pair.local, "origin", &branch, NetHooks::default()).unwrap();

    assert_eq!(head_oid(&pair.local), pushed);
    assert_eq!(read_file(&pair.local, "shared.txt"), "shared v2");
    assert_eq!(read_file(&pair.local, "local.txt"), "local edit");
    assert_eq!(pair.local.state(), git2::RepositoryState::Clean);
}

#[test]
fn test_pull_refuses_to_overwrite_local_changes() {
    let pair = setup_pair();
    other_pushes(&pair, "shared.txt", "shared v2");
    write_file(&pair.local, "shared.txt", "uncommitted work");
    let before = head_oid(&pair.local);
    let branch = current_branch(&pair.local);

    let err = pull(&pair.local, "origin", &branch, NetHooks::default()).unwrap_err();

    assert_eq!(err.to_string(), t("git_error_local_changes_overwritten"));
    assert_eq!(read_file(&pair.local, "shared.txt"), "uncommitted work");
    assert_eq!(head_oid(&pair.local), before);
}

#[test]
fn test_pull_normal_merge_records_both_parents() {
    let pair = setup_pair();
    let theirs = other_pushes(&pair, "theirs.txt", "theirs");
    let ours = write_and_commit(&pair.local, "ours.txt", "ours", "local work");
    let branch = current_branch(&pair.local);

    pull(&pair.local, "origin", &branch, NetHooks::default()).unwrap();

    let head = pair.local.head().unwrap().peel_to_commit().unwrap();
    let parents: Vec<git2::Oid> = head.parent_ids().collect();
    assert_eq!(parents, vec![ours, theirs]);
    assert_eq!(read_file(&pair.local, "theirs.txt"), "theirs");
    assert_eq!(read_file(&pair.local, "ours.txt"), "ours");
    assert_eq!(pair.local.state(), git2::RepositoryState::Clean);
    assert!(get_status(&pair.local).is_empty());
}

#[test]
fn test_pull_with_conflicts_leaves_the_merge_in_progress() {
    let pair = setup_pair();
    other_pushes(&pair, "shared.txt", "theirs\n");
    write_and_commit(&pair.local, "shared.txt", "ours\n", "local work");
    let branch = current_branch(&pair.local);

    let outcome = pull(&pair.local, "origin", &branch, NetHooks::default()).unwrap();

    assert_eq!(outcome, Outcome::Conflicts(vec!["shared.txt".into()]));
    assert_eq!(pair.local.state(), git2::RepositoryState::Merge);
    let marked = read_file(&pair.local, "shared.txt");
    assert!(
        marked.contains("<<<<<<<") && marked.contains("theirs"),
        "{marked}"
    );
    let status = get_status(&pair.local);
    assert!(status.iter().any(|e| e.kind == StatusKind::Conflicted));

    abort_merge(&pair.local).unwrap();
    assert_eq!(read_file(&pair.local, "shared.txt").trim_end(), "ours");
    assert_eq!(pair.local.state(), git2::RepositoryState::Clean);
}

#[test]
fn test_pull_uses_configured_upstream() {
    let pair = setup_pair();
    create_branch(&pair.other, "published", "HEAD").unwrap();
    checkout_branch(&pair.other, "published").unwrap();
    let pushed = other_pushes(&pair, "shared.txt", "published");

    fetch(&pair.local, "origin", NetHooks::default()).unwrap();
    create_branch(&pair.local, "topic", "HEAD").unwrap();
    checkout_branch(&pair.local, "topic").unwrap();
    pair.local
        .find_branch("topic", git2::BranchType::Local)
        .unwrap()
        .set_upstream(Some("origin/published"))
        .unwrap();

    // There is no origin/topic: only the configured upstream can satisfy this.
    pull(&pair.local, "origin", "topic", NetHooks::default()).unwrap();

    assert_eq!(head_oid(&pair.local), pushed);
    assert_eq!(read_file(&pair.local, "shared.txt"), "published");
}

#[test]
fn test_push_non_fast_forward_is_an_error() {
    let pair = setup_pair();
    other_pushes(&pair, "theirs.txt", "theirs");
    write_and_commit(&pair.local, "ours.txt", "ours", "diverged");
    let branch = current_branch(&pair.local);

    assert!(push(&pair.local, "origin", &branch, false, NetHooks::default()).is_err());
}

#[test]
fn test_push_rejected_reference_is_reported() {
    let pair = setup_pair();
    create_branch(&pair.local, "locked", "HEAD").unwrap();
    // A held ref lock makes the remote fail to update `locked`; the transport
    // reports that only through the per-reference status, not as an error.
    std::fs::write(pair.remote_dir.path().join("refs/heads/locked.lock"), "").unwrap();

    let err = push(&pair.local, "origin", "locked", false, NetHooks::default()).unwrap_err();

    let expected_prefix = t("git_error_push_rejected").replace("{}", "");
    assert!(err.to_string().starts_with(&expected_prefix), "{err}");
}

#[test]
fn test_push_goes_to_the_tracked_branch_even_when_named_differently() {
    let pair = setup_pair();
    let (remote, branch) = (&pair.local, current_branch(&pair.local));
    create_branch(remote, "feat", "HEAD").unwrap();
    checkout_branch(remote, "feat").unwrap();
    // `feat` tracks `origin/feature-x` (like `git push -u origin feat:feature-x`).
    let mut cfg = remote.config().unwrap();
    cfg.set_str("branch.feat.remote", "origin").unwrap();
    cfg.set_str("branch.feat.merge", "refs/heads/feature-x")
        .unwrap();
    let oid = write_and_commit(remote, "feat.txt", "feat", "feat work");

    push(remote, "origin", "feat", false, NetHooks::default()).unwrap();

    let bare = git2::Repository::open_bare(pair.remote_dir.path()).unwrap();
    assert_eq!(
        bare.find_reference("refs/heads/feature-x")
            .unwrap()
            .target(),
        Some(oid)
    );
    assert!(bare.find_reference("refs/heads/feat").is_err());
    assert_ne!(branch, "feat");
}
