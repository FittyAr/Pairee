//! Merge / cherry-pick / revert / rebase state handling and stash pop.

use super::*;
use crate::config::localization::t;

#[test]
fn index_paths_keep_their_bytes() {
    use crate::git::outcome::index_path;
    assert_eq!(index_path(b"dir/a.txt"), std::path::Path::new("dir/a.txt"));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let raw = b"caf\xe9.txt";
        assert_eq!(index_path(raw).as_os_str().as_bytes(), raw, "not lossy");
    }
}

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
    assert!(get_status(&repo).unwrap().is_empty());
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

    let oid = stash_oid(&mut repo, 0);
    stash_pop(&mut repo, &oid).unwrap();

    assert_eq!(read_file(&repo, "file.txt"), "work in progress");
    assert!(list_stashes(&mut repo).unwrap().is_empty());
}

#[test]
fn test_merge_like_operations_refuse_staged_changes_and_keep_them() {
    let (_dir, repo, _main, feature) = repo_with_feature();
    write_and_commit(&repo, "main.txt", "main", "main work");
    write_file(&repo, "staged.txt", "staged work");
    stage_file(&repo, "staged.txt").unwrap();
    let before = repo.head().unwrap().target();

    for err in [
        merge(&repo, "feature").unwrap_err(),
        cherry_pick(&repo, &feature.to_string()).unwrap_err(),
        revert(&repo, &before.unwrap().to_string()).unwrap_err(),
    ] {
        assert_eq!(err.to_string(), t("git_error_staged_changes"));
    }
    assert_eq!(repo.head().unwrap().target(), before);
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    assert_eq!(read_file(&repo, "staged.txt"), "staged work");
    assert!(
        get_status(&repo)
            .unwrap()
            .iter()
            .any(|s| s.path == "staged.txt" && s.is_staged)
    );
}

#[test]
fn test_no_new_operation_while_one_is_pending() {
    let (_dir, repo) = repo_with_conflict();
    let feature = repo
        .find_branch("feature", git2::BranchType::Local)
        .unwrap()
        .get()
        .target()
        .unwrap();
    assert!(matches!(
        merge(&repo, "feature").unwrap(),
        Outcome::Conflicts(_)
    ));
    write_file(&repo, "conflict.txt", "resolved\n");
    stage_file(&repo, "conflict.txt").unwrap();

    let err = cherry_pick(&repo, &feature.to_string()).unwrap_err();
    assert_eq!(err.to_string(), t("git_error_operation_pending"));
    assert!(commit_amend(&repo, "amend", "", "").is_err());
    assert_eq!(PendingOperation::of(&repo), Some(PendingOperation::Merge));
}

#[test]
fn test_stash_pop_with_conflicts_keeps_the_stash() {
    let (_dir, mut repo) = setup_temp_repo();
    write_and_commit(&repo, "file.txt", "base\n", "base");
    write_file(&repo, "file.txt", "stashed\n");
    stage_file(&repo, "file.txt").unwrap();
    stash_save(&mut repo, Some("wip"), false).unwrap();
    write_and_commit(&repo, "file.txt", "committed later\n", "later");
    let oid = stash_oid(&mut repo, 0);

    let result = stash_pop(&mut repo, &oid);

    if repo.index().unwrap().has_conflicts() {
        let err = result.unwrap_err().to_string();
        assert!(err.contains("file.txt"), "{err}");
    } else {
        // libgit2 refused to apply over the conflicting change.
        assert!(result.is_err());
    }
    assert_eq!(list_stashes(&mut repo).unwrap().len(), 1, "stash kept");
}

#[test]
fn test_stash_drop_follows_the_commit_id_not_a_stale_index() {
    let (_dir, mut repo) = setup_temp_repo();
    write_and_commit(&repo, "file.txt", "base", "base");
    write_file(&repo, "file.txt", "first");
    stash_save(&mut repo, Some("first"), false).unwrap();
    let first = stash_oid(&mut repo, 0);
    // Another stash shifts `first` to stash@{1}.
    write_file(&repo, "file.txt", "second");
    stash_save(&mut repo, Some("second"), false).unwrap();

    stash_drop(&mut repo, &first).unwrap();

    let left = list_stashes(&mut repo).unwrap();
    assert_eq!(left.len(), 1);
    assert!(left[0].message.contains("second"), "{}", left[0].message);
    assert_eq!(
        stash_drop(&mut repo, &first).unwrap_err().to_string(),
        t("git_error_stash_gone")
    );
}

#[test]
fn a_broken_index_is_an_error_not_a_clean_status() {
    let (dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "a.txt", "a", "first");
    std::fs::write(repo.path().join("index"), b"DIRC garbage, not an index").unwrap();
    let repo = git2::Repository::open(dir.path()).unwrap();
    assert!(get_status(&repo).is_err());

    let snapshot = crate::git::snapshot::load(dir.path(), 10).unwrap();
    assert!(snapshot.status.is_empty());
    assert!(
        snapshot.status_error.is_some(),
        "the panel must not show it clean"
    );
}

#[test]
fn test_abort_merge_without_operation_is_refused() {
    let (_dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "a.txt", "1", "one");
    write_file(&repo, "b.txt", "new");
    stage_file(&repo, "b.txt").unwrap();

    assert!(abort_merge(&repo).is_err());
    // Nothing was touched.
    assert_eq!(read_file(&repo, "b.txt"), "new");
    assert!(
        get_status(&repo)
            .unwrap()
            .iter()
            .any(|s| s.path == "b.txt" && s.is_staged)
    );
}

#[test]
fn test_status_reports_a_staged_rename_and_unstages_both_sides() {
    let (_dir, repo) = setup_temp_repo();
    let body = "some reasonably long content\nwith several lines\nto be detected\n";
    write_and_commit(&repo, "old.txt", body, "add");
    std::fs::rename(
        repo.workdir().unwrap().join("old.txt"),
        repo.workdir().unwrap().join("new.txt"),
    )
    .unwrap();
    stage_file(&repo, "new.txt").unwrap();
    stage_file(&repo, "old.txt").unwrap();

    let status = get_status(&repo).unwrap();
    assert_eq!(status.len(), 1, "{status:?}");
    assert_eq!(status[0].kind, StatusKind::Renamed);
    assert_eq!(status[0].path, "new.txt");

    unstage_file(&repo, "new.txt").unwrap();
    let status = get_status(&repo).unwrap();
    assert!(status.iter().all(|s| !s.is_staged), "{status:?}");
}

#[cfg(unix)]
#[test]
fn test_discard_removes_a_dangling_untracked_symlink() {
    let (dir, repo) = setup_temp_repo();
    write_and_commit(&repo, "a.txt", "1", "one");
    let link = dir.path().join("dangling");
    std::os::unix::fs::symlink("missing-target", &link).unwrap();

    discard_file_changes(&repo, "dangling").unwrap();

    assert!(link.symlink_metadata().is_err());
}
