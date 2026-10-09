use super::*;

#[test]
fn test_get_file_diff_staged() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "initial version\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    // Modify and stage
    std::fs::write(&file_path, "modified version\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();

    let diff = get_file_diff(&repo, "file.txt", true).unwrap();
    assert!(diff.contains("-initial version"));
    assert!(diff.contains("+modified version"));
}

#[test]
fn test_get_file_diff_unstaged() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "initial version\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    // Modify without staging
    std::fs::write(&file_path, "modified unstaged\n").unwrap();

    let diff = get_file_diff(&repo, "file.txt", false).unwrap();
    assert!(diff.contains("-initial version"));
    assert!(diff.contains("+modified unstaged"));
}

#[test]
fn test_get_file_diff_untracked() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("untracked_new.txt");
    std::fs::write(&file_path, "brand new untracked content\n").unwrap();

    let diff = get_file_diff(&repo, "untracked_new.txt", false).unwrap();
    assert!(diff.contains("+brand new untracked content"));
}

#[test]
fn test_get_commit_diff() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "line 1\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    std::fs::write(&file_path, "line 1\nline 2\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    let c2_oid = commit(&repo, "c2", "Test User", "test@example.com").unwrap();

    let diff = get_commit_diff(&repo, &c2_oid.to_string()).unwrap();
    assert!(diff.contains("+line 2"));
}

#[test]
fn test_get_stash_diff() {
    let (dir, mut repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "initial\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    std::fs::write(&file_path, "stashed changes\n").unwrap();

    stash_save(&mut repo, Some("my test stash"), false).unwrap();
    let stashes = list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 1);

    let diff = get_stash_diff(&repo, &stashes[0].oid).unwrap();
    assert!(diff.contains("+stashed changes"));
}

#[test]
fn test_get_file_diff_binary() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("image.png");
    std::fs::write(&file_path, [0x89, b'P', b'N', b'G', 0x00, 0x01, 0x02]).unwrap();
    stage_file(&repo, "image.png").unwrap();
    commit(&repo, "add binary", "Test User", "test@example.com").unwrap();

    std::fs::write(&file_path, [0x89, b'P', b'N', b'G', 0x00, 0x05, 0x06]).unwrap();
    let diff = get_file_diff(&repo, "image.png", false).unwrap();
    assert!(diff.contains("Binary files"));
}

#[test]
fn test_get_file_diff_binary_untracked() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("untracked_image.png");
    std::fs::write(&file_path, [0x89, b'P', b'N', b'G', 0x00, 0x01, 0x02]).unwrap();
    let diff = get_file_diff(&repo, "untracked_image.png", false).unwrap();
    assert!(
        diff.contains("Binary files")
            || diff == crate::config::localization::t("git_diff_binary_file")
    );
}

#[test]
fn test_cannot_delete_current_branch() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "hello\n").unwrap();
    stage_file(&repo, "file.txt").unwrap();
    commit(&repo, "initial", "Test User", "test@example.com").unwrap();

    let branches = get_branches(&repo);
    let current_branch = branches.iter().find(|b| b.is_current).unwrap();
    assert!(current_branch.is_current);

    // libgit2 itself prevents deleting the checked-out branch
    let res = delete_branch(&repo, &current_branch.name);
    assert!(res.is_err());
}

#[test]
fn test_unborn_branch_resolution() {
    let (_dir, repo) = setup_temp_repo();
    assert!(!repo.head_detached().unwrap_or(true));

    let head_ref = repo.find_reference("HEAD").unwrap();
    let target = head_ref.symbolic_target().ok().flatten().unwrap();
    let branch_name = target.strip_prefix("refs/heads/").unwrap();
    assert!(!branch_name.is_empty());
    assert_ne!(branch_name, "detached HEAD");
}

#[test]
fn test_merge_and_reset_confirmation_formatting() {
    let msg_merge = crate::config::localization::t("git_confirm_merge_branch")
        .replace("{source}", "feature/new")
        .replace("{target}", "main");
    assert!(msg_merge.contains("feature/new"));
    assert!(msg_merge.contains("main"));

    let mode_soft = crate::config::localization::t("git_reset_mode_soft");
    let msg_reset = crate::config::localization::t("git_confirm_reset")
        .replace("{commit}", "1234abc")
        .replace("{mode}", &mode_soft);
    assert!(msg_reset.contains("1234abc"));
    assert!(msg_reset.contains(&mode_soft));
}
