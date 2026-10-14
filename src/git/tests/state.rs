//! Merge / cherry-pick / revert / rebase state handling and stash pop.

use super::*;
use crate::config::localization::t;

/// A repo on its default branch with `base.txt`, plus a `feature` branch one
/// commit ahead that adds `feature.txt`. Returns the default branch name.
fn repo_with_feature() -> (TempDir, git2::Repository, String, git2::Oid) {
    let (dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "base.txt", "base", "base");
    let main = current_branch(&repo);
    create_branch(&repo, "feature", "HEAD").unwrap();
    checkout_branch(&repo, "feature").unwrap();
    let feature = write_and_commit(&repo, "feature.txt", "feature", "feature work");
    checkout_branch(&repo, &main).unwrap();
    (dir, repo, main, feature)
}

/// `main` and `feature` both change `conflict.txt` after a common base.
fn repo_with_conflict() -> (TempDir, git2::Repository) {
    let (dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "conflict.txt", "base\n", "base");
    let main = current_branch(&repo);
    create_branch(&repo, "feature", "HEAD").unwrap();
    checkout_branch(&repo, "feature").unwrap();
    write_and_commit(&repo, "conflict.txt", "feature\n", "feature");
    checkout_branch(&repo, &main).unwrap();
    write_and_commit(&repo, "conflict.txt", "main\n", "main");
    (dir, repo)
}

#[test]
fn test_clean_merge_is_committed_with_both_parents() {
    let (_dir, repo, _main, feature) = repo_with_feature();
    let ours = write_and_commit(&repo, "main.txt", "main", "main work");

    let Outcome::Committed(merge_oid) = merge(&repo, "feature").unwrap() else {
        panic!("a clean merge is committed");
    };

    let merged = repo.find_commit(merge_oid).unwrap();
    assert_eq!(merged.parent_ids().collect::<Vec<_>>(), vec![ours, feature]);
    assert_eq!(merged.summary().unwrap(), Some("Merge branch 'feature'"));
    assert_eq!(repo.head().unwrap().target(), Some(merge_oid));
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert!(!repo.path().join("MERGE_HEAD").exists());
    assert!(get_status(&repo).is_empty());
}

#[test]
fn test_conflicted_merge_stays_in_progress_until_resolved_and_committed() {
    let (_dir, repo) = repo_with_conflict();

    let outcome = merge(&repo, "feature").unwrap();

    assert_eq!(outcome, Outcome::Conflicts(vec!["conflict.txt".into()]));
    assert_eq!(PendingOperation::of(&repo), Some(PendingOperation::Merge));
    let marked = read_file(&repo, "conflict.txt");
    assert!(
        marked.contains("<<<<<<<") && marked.contains(">>>>>>>"),
        "{marked}"
    );

    // Resolve, mark resolved (stage) and commit: both parents recorded.
    write_file(&repo, "conflict.txt", "resolved\n");
    stage_file(&repo, "conflict.txt").unwrap();
    assert!(conflicted_paths(&repo).unwrap().is_empty());
    let oid = commit(&repo, "Merge feature", "", "").unwrap();

    assert_eq!(repo.find_commit(oid).unwrap().parent_count(), 2);
    assert_eq!(PendingOperation::of(&repo), None);
}

#[test]
fn test_fast_forward_merge_refuses_to_overwrite_local_changes() {
    let (_dir, repo, _main, _feature) = repo_with_feature();
    write_file(&repo, "feature.txt", "untracked work in the way");
    let before = repo.head().unwrap().target();

    let err = merge(&repo, "feature").unwrap_err();

    assert_eq!(err.to_string(), t("git_error_local_changes_overwritten"));
    assert_eq!(read_file(&repo, "feature.txt"), "untracked work in the way");
    assert_eq!(repo.head().unwrap().target(), before);
}

#[test]
fn test_clean_cherry_pick_is_committed_with_the_original_message_and_author() {
    let (_dir, repo, _main, feature) = repo_with_feature();

    let Outcome::Committed(oid) = cherry_pick(&repo, &feature.to_string()).unwrap() else {
        panic!("a clean cherry-pick is committed");
    };

    let picked = repo.find_commit(oid).unwrap();
    let original = repo.find_commit(feature).unwrap();
    assert_eq!(picked.parent_count(), 1);
    assert_eq!(picked.message(), original.message());
    assert_eq!(picked.author().email(), original.author().email());
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert_eq!(read_file(&repo, "feature.txt"), "feature");
}

#[test]
fn test_conflicted_cherry_pick_waits_and_can_be_aborted() {
    let (_dir, repo) = repo_with_conflict();
    let feature = repo
        .find_branch("feature", git2::BranchType::Local)
        .unwrap()
        .get()
        .target()
        .unwrap();
    let before = repo.head().unwrap().target();

    let outcome = cherry_pick(&repo, &feature.to_string()).unwrap();

    assert_eq!(outcome, Outcome::Conflicts(vec!["conflict.txt".into()]));
    assert_eq!(
        PendingOperation::of(&repo),
        Some(PendingOperation::CherryPick)
    );
    abort_merge(&repo).unwrap();
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert_eq!(read_file(&repo, "conflict.txt").trim_end(), "main");
    assert_eq!(repo.head().unwrap().target(), before);
}

#[test]
fn test_clean_revert_is_committed() {
    let (_dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "file.txt", "v1", "v1");
    let v2 = write_and_commit(&repo, "file.txt", "v2", "v2");

    let Outcome::Committed(oid) = revert(&repo, &v2.to_string()).unwrap() else {
        panic!("a clean revert is committed");
    };

    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert_eq!(read_file(&repo, "file.txt"), "v1");
    let message = repo
        .find_commit(oid)
        .unwrap()
        .message()
        .unwrap()
        .to_string();
    assert!(message.starts_with("Revert \"v2\""), "{message}");
}

#[test]
fn test_abort_merge_keeps_unrelated_local_changes() {
    let (_dir, repo) = setup_temp_repo();
    write_file(&repo, "notes.txt", "notes v1");
    stage_file(&repo, "notes.txt").unwrap();
    write_and_commit(&repo, "conflict.txt", "base", "base");
    let main = current_branch(&repo);

    create_branch(&repo, "feature", "HEAD").unwrap();
    checkout_branch(&repo, "feature").unwrap();
    write_file(&repo, "added.txt", "added by feature");
    stage_file(&repo, "added.txt").unwrap();
    write_and_commit(&repo, "conflict.txt", "feature", "feature");
    checkout_branch(&repo, &main).unwrap();
    write_and_commit(&repo, "conflict.txt", "main", "main");

    write_file(&repo, "notes.txt", "uncommitted notes");
    merge(&repo, "feature").unwrap();
    assert!(repo.index().unwrap().has_conflicts());

    abort_merge(&repo).unwrap();

    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert_eq!(read_file(&repo, "conflict.txt"), "main");
    assert_eq!(read_file(&repo, "notes.txt"), "uncommitted notes");
    assert!(!repo.workdir().unwrap().join("added.txt").exists());
    assert!(!repo.index().unwrap().has_conflicts());
}

#[test]
fn test_rebase_skips_already_applied_commit() {
    let (_dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "base.txt", "base", "base");
    let main = current_branch(&repo);
    create_branch(&repo, "feat", "HEAD").unwrap();

    // The same change lands on both branches as different commits.
    write_and_commit(&repo, "same.txt", "same", "on main");
    checkout_branch(&repo, "feat").unwrap();
    write_and_commit(&repo, "same.txt", "same", "on feat");
    write_and_commit(&repo, "feat.txt", "feat", "feat only");

    rebase_branch(&repo, &main).unwrap();

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.message().unwrap(), "feat only");
    let parent = head.parent(0).unwrap();
    assert_eq!(parent.message().unwrap(), "on main");
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
}

#[test]
fn test_stash_pop_restores_and_drops() {
    let (_dir, mut repo) = setup_temp_repo();
    write_and_commit(&repo, "file.txt", "committed", "base");
    write_file(&repo, "file.txt", "work in progress");
    stash_save(&mut repo, Some("wip"), false).unwrap();
    assert_eq!(read_file(&repo, "file.txt"), "committed");

    stash_pop(&mut repo, 0).unwrap();

    assert_eq!(read_file(&repo, "file.txt"), "work in progress");
    assert!(list_stashes(&mut repo).unwrap().is_empty());
}
