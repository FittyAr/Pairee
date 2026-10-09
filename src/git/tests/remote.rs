use super::*;

#[test]
fn test_remote_list_add_delete() {
    let (_dir, repo) = setup_temp_repo();

    let remotes = list_remotes(&repo).unwrap();
    assert!(remotes.is_empty());

    add_remote(&repo, "upstream", "https://example.com/upstream.git").unwrap();
    add_remote(&repo, "origin", "https://example.com/origin.git").unwrap();

    let remotes = list_remotes(&repo).unwrap();
    assert_eq!(remotes.len(), 2);
    // Should be sorted alphabetically
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(
        remotes[0].url.as_deref(),
        Some("https://example.com/origin.git")
    );
    assert_eq!(remotes[1].name, "upstream");
    assert_eq!(
        remotes[1].url.as_deref(),
        Some("https://example.com/upstream.git")
    );

    delete_remote(&repo, "upstream").unwrap();
    let remotes = list_remotes(&repo).unwrap();
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].name, "origin");
}

#[test]
fn test_resolve_remote_name() {
    let (_dir, repo) = setup_temp_repo();

    // No remotes configured
    assert!(resolve_remote_name(&repo, None).is_err());

    // Add a custom remote
    add_remote(&repo, "custom", "https://example.com/custom.git").unwrap();
    assert_eq!(resolve_remote_name(&repo, None).unwrap(), "custom");

    // Add origin: now origin takes precedence when no upstream branch
    add_remote(&repo, "origin", "https://example.com/origin.git").unwrap();
    assert_eq!(resolve_remote_name(&repo, None).unwrap(), "origin");
}

#[test]
fn test_remote_push_upstream_and_delete_branch() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("test.txt");
    File::create(&file_path)
        .unwrap()
        .write_all(b"initial")
        .unwrap();
    stage_file(&repo, "test.txt").unwrap();
    commit(&repo, "initial commit", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    // Initialize a local bare repository to act as our remote
    let remote_dir = TempDir::new().unwrap();
    let bare_path = remote_dir.path().to_str().unwrap().replace('\\', "/");
    let remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();

    add_remote(&repo, "test-remote", &bare_path).unwrap();

    // Push default branch with upstream
    push(&repo, "test-remote", &default_branch, true, None).unwrap();

    let local_head_branch = repo
        .find_branch(&default_branch, git2::BranchType::Local)
        .unwrap();
    let upstream = local_head_branch.upstream().unwrap();
    assert_eq!(
        upstream.name().unwrap(),
        Some(format!("test-remote/{}", default_branch).as_str())
    );

    // Create a new branch, push it, and then delete it remotely
    create_branch(&repo, "feat-to-delete", "HEAD").unwrap();
    push(&repo, "test-remote", "feat-to-delete", true, None).unwrap();

    // Remote bare repo must have the branch
    assert!(
        remote_repo
            .find_branch("feat-to-delete", git2::BranchType::Local)
            .is_ok()
    );

    // Delete remote branch
    delete_remote_branch(&repo, "test-remote", "feat-to-delete", None).unwrap();

    // Verify remote bare repo no longer has the branch
    assert!(
        remote_repo
            .find_branch("feat-to-delete", git2::BranchType::Local)
            .is_err()
    );
}

#[test]
fn test_log_paged() {
    let (dir, repo) = setup_temp_repo();
    for i in 1..=5 {
        let file_path = dir.path().join(format!("file_{}.txt", i));
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "content {}", i).unwrap();
        stage_file(&repo, &format!("file_{}.txt", i)).unwrap();
        commit(
            &repo,
            &format!("commit {}", i),
            "Test User",
            "test@example.com",
        )
        .unwrap();
    }

    let paged1 = get_log_paged(&repo, 0, 2);
    assert_eq!(paged1.len(), 2);
    assert_eq!(paged1[0].message, "commit 5");
    assert_eq!(paged1[1].message, "commit 4");

    let paged2 = get_log_paged(&repo, 2, 2);
    assert_eq!(paged2.len(), 2);
    assert_eq!(paged2[0].message, "commit 3");
    assert_eq!(paged2[1].message, "commit 2");

    let paged3 = get_log_paged(&repo, 4, 2);
    assert_eq!(paged3.len(), 1);
    assert_eq!(paged3[0].message, "commit 1");

    let paged_empty = get_log_paged(&repo, 10, 2);
    assert_eq!(paged_empty.len(), 0);
}

#[test]
fn test_create_tag() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("tag_test.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "tag content").unwrap();
    stage_file(&repo, "tag_test.txt").unwrap();
    let oid = commit(&repo, "tag commit", "Test User", "test@example.com").unwrap();

    // Lightweight tag
    let tag_oid = create_tag(&repo, "v1.0.0", &oid.to_string(), None).unwrap();
    assert_eq!(tag_oid, oid);
    assert!(repo.find_reference("refs/tags/v1.0.0").is_ok());

    // Annotated tag
    let tag_annotated_oid = create_tag(&repo, "v2.0.0", "HEAD", Some("annotated release")).unwrap();
    assert!(!tag_annotated_oid.is_zero());
    assert!(repo.find_reference("refs/tags/v2.0.0").is_ok());
}

#[test]
fn test_cherry_pick() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("base.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "base").unwrap();
    stage_file(&repo, "base.txt").unwrap();
    commit(&repo, "base commit", "Test User", "test@example.com").unwrap();

    let main_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    // Create feature branch and commit
    create_branch(&repo, "feat-cherry", "HEAD").unwrap();
    checkout_branch(&repo, "feat-cherry").unwrap();

    let feat_path = dir.path().join("cherry.txt");
    let mut f2 = File::create(&feat_path).unwrap();
    writeln!(f2, "cherry content").unwrap();
    stage_file(&repo, "cherry.txt").unwrap();
    let feat_oid = commit(&repo, "cherry commit", "Test User", "test@example.com").unwrap();

    // Switch back to main branch
    checkout_branch(&repo, &main_branch).unwrap();
    assert!(!dir.path().join("cherry.txt").exists());

    // Cherry-pick commit from feature branch
    cherry_pick(&repo, &feat_oid.to_string()).unwrap();

    // The cherry-picked changes should be staged in the index
    let staged = get_status(&repo);
    let found = staged.iter().any(|s| s.path == "cherry.txt" && s.is_staged);
    assert!(found);
}

#[test]
fn test_revert() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("revert.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "v1").unwrap();
    stage_file(&repo, "revert.txt").unwrap();
    commit(&repo, "init revert file", "Test User", "test@example.com").unwrap();

    // Commit change
    let mut f2 = File::create(&file_path).unwrap();
    writeln!(f2, "v2").unwrap();
    stage_file(&repo, "revert.txt").unwrap();
    let v2_oid = commit(&repo, "update to v2", "Test User", "test@example.com").unwrap();

    // Revert v2
    revert(&repo, &v2_oid.to_string()).unwrap();

    // Index should now have revert changes
    let status = get_status(&repo);
    let found = status.iter().any(|s| s.path == "revert.txt" && s.is_staged);
    assert!(found);
}

#[test]
fn test_stash_clear_and_diff() {
    let (dir, mut repo) = setup_temp_repo();
    let file_path = dir.path().join("stash_diff.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "initial").unwrap();
    stage_file(&repo, "stash_diff.txt").unwrap();
    commit(&repo, "base", "Test User", "test@example.com").unwrap();

    // Modify file and stash
    let mut f2 = File::create(&file_path).unwrap();
    writeln!(f2, "modified content for stash").unwrap();
    stash_save(&mut repo, Some("diff test stash"), false).unwrap();

    let stashes = list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 1);

    let diff = get_stash_diff(&repo, &stashes[0].oid).unwrap();
    assert!(diff.contains("+modified content for stash"));

    // Clear stashes
    stash_clear(&mut repo).unwrap();
    let stashes_after = list_stashes(&mut repo).unwrap();
    assert_eq!(stashes_after.len(), 0);
}

#[test]
fn test_stash_untracked() {
    let (dir, mut repo) = setup_temp_repo();
    let file_path = dir.path().join("base.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "base").unwrap();
    stage_file(&repo, "base.txt").unwrap();
    commit(&repo, "base", "Test User", "test@example.com").unwrap();

    // Create untracked file
    let untracked_path = dir.path().join("untracked.txt");
    let mut f_untracked = File::create(&untracked_path).unwrap();
    writeln!(f_untracked, "secret untracked content").unwrap();

    // Stash with untracked = true
    stash_save(&mut repo, Some("stash with untracked"), true).unwrap();
    assert!(!untracked_path.exists());

    // Apply stash
    stash_apply(&mut repo, 0).unwrap();
    assert!(untracked_path.exists());
}

#[test]
fn test_rebase_branch() {
    let (dir, repo) = setup_temp_repo();
    let base_file = dir.path().join("base.txt");
    let mut f = File::create(&base_file).unwrap();
    writeln!(f, "base").unwrap();
    stage_file(&repo, "base.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    // Create feature branch
    create_branch(&repo, "feat", "HEAD").unwrap();
    checkout_branch(&repo, "feat").unwrap();

    let feat_file = dir.path().join("feat.txt");
    let mut f_feat = File::create(&feat_file).unwrap();
    writeln!(f_feat, "feat").unwrap();
    stage_file(&repo, "feat.txt").unwrap();
    commit(&repo, "c2_feat", "Test User", "test@example.com").unwrap();

    // Checkout default branch and add a commit
    checkout_branch(&repo, &default_branch).unwrap();
    let main_file = dir.path().join("main.txt");
    let mut f_main = File::create(&main_file).unwrap();
    writeln!(f_main, "main").unwrap();
    stage_file(&repo, "main.txt").unwrap();
    commit(&repo, "c3_main", "Test User", "test@example.com").unwrap();

    // Rebase feat onto default branch
    checkout_branch(&repo, "feat").unwrap();
    rebase_branch(&repo, &default_branch).unwrap();

    // After rebase, feat branch must contain both feat.txt and main.txt
    assert!(dir.path().join("feat.txt").exists());
    assert!(dir.path().join("main.txt").exists());
}

#[test]
fn test_branches_ahead_behind() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("initial.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "init").unwrap();
    stage_file(&repo, "initial.txt").unwrap();
    commit(&repo, "initial commit", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    // Setup local bare remote
    let remote_dir = TempDir::new().unwrap();
    let bare_path = remote_dir.path().to_str().unwrap().replace('\\', "/");
    let _remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();

    add_remote(&repo, "origin", &bare_path).unwrap();
    push(&repo, "origin", &default_branch, true, None).unwrap();

    // Initially ahead = 0, behind = 0
    let branches = get_branches(&repo);
    let current = branches.iter().find(|b| b.name == default_branch).unwrap();
    assert_eq!(current.ahead, 0);
    assert_eq!(current.behind, 0);

    // Make local commit without pushing
    let f2_path = dir.path().join("ahead.txt");
    let mut f2 = File::create(&f2_path).unwrap();
    writeln!(f2, "ahead").unwrap();
    stage_file(&repo, "ahead.txt").unwrap();
    commit(&repo, "ahead commit", "Test User", "test@example.com").unwrap();

    // Now ahead should be 1, behind 0
    let branches_updated = get_branches(&repo);
    let current_updated = branches_updated
        .iter()
        .find(|b| b.name == default_branch)
        .unwrap();
    assert_eq!(current_updated.ahead, 1);
    assert_eq!(current_updated.behind, 0);
}

#[test]
fn test_list_and_delete_tags() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("t.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "t").unwrap();
    stage_file(&repo, "t.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    create_tag(&repo, "v0.1.0", "HEAD", None).unwrap();
    create_tag(&repo, "v0.2.0", "HEAD", Some("beta release")).unwrap();

    let tags = list_tags(&repo).unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "v0.1.0");
    assert_eq!(tags[0].message, None);
    assert_eq!(tags[1].name, "v0.2.0");
    assert!(
        tags[1]
            .message
            .as_deref()
            .unwrap_or("")
            .contains("beta release")
    );

    delete_tag(&repo, "v0.1.0").unwrap();
    let tags_after = list_tags(&repo).unwrap();
    assert_eq!(tags_after.len(), 1);
    assert_eq!(tags_after[0].name, "v0.2.0");
}

#[test]
fn test_push_tags() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("t.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "t").unwrap();
    stage_file(&repo, "t.txt").unwrap();
    commit(&repo, "c1", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    let remote_dir = TempDir::new().unwrap();
    let bare_path = remote_dir.path().to_str().unwrap().replace('\\', "/");
    let remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();

    add_remote(&repo, "origin", &bare_path).unwrap();
    push(&repo, "origin", &default_branch, true, None).unwrap();

    create_tag(&repo, "v1.5.0", "HEAD", None).unwrap();
    push_tags(&repo, "origin", None).unwrap();

    // Verify remote bare repository received the tag
    assert!(remote_repo.find_reference("refs/tags/v1.5.0").is_ok());
}

#[test]
fn test_init_repo() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path()).unwrap();
    assert!(!repo.is_bare());
    assert!(dir.path().join(".git").exists());
    assert!(find_repo(dir.path()).is_some());
}

#[test]
fn test_clone_repo() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("cloned_file.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "hello clone").unwrap();
    stage_file(&repo, "cloned_file.txt").unwrap();
    let src_oid = commit(
        &repo,
        "initial clone commit",
        "Test User",
        "test@example.com",
    )
    .unwrap();

    let target_dir = TempDir::new().unwrap();
    let clone_target_path = target_dir.path().join("cloned_sub");
    let src_url = dir.path().to_str().unwrap().replace('\\', "/");

    let cloned_repo = clone_repo(&src_url, &clone_target_path, None).unwrap();
    assert!(clone_target_path.join("cloned_file.txt").exists());

    let content = std::fs::read_to_string(clone_target_path.join("cloned_file.txt")).unwrap();
    assert!(content.contains("hello clone"));

    let cloned_head = cloned_repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(cloned_head.id(), src_oid);
}
