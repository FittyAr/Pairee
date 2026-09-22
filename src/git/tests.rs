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

#[test]
fn test_repo_init_and_discover() {
    let (dir, repo) = setup_temp_repo();
    let workdir = get_workdir(&repo).unwrap();
    assert_eq!(
        std::fs::canonicalize(workdir).unwrap(),
        std::fs::canonicalize(dir.path()).unwrap()
    );

    let discovered = find_repo(dir.path()).unwrap();
    assert_eq!(discovered.path(), repo.path());
}

#[test]
fn test_stage_unstage_and_commit() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("test.txt");

    // Write file
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "hello world").unwrap();
    }

    // Stage file
    stage_file(&repo, "test.txt").unwrap();

    // Get status via diff
    let diff = get_file_diff(&repo, "test.txt", true).unwrap();
    assert!(diff.contains("+hello world"));

    // Commit file
    let oid = commit(&repo, "initial commit", "Test User", "test@example.com").unwrap();
    assert!(!oid.to_string().is_empty());

    // Modify file
    {
        let mut f = File::options().append(true).open(&file_path).unwrap();
        writeln!(f, "new line").unwrap();
    }

    // Diff unstaged
    let diff_unstaged = get_file_diff(&repo, "test.txt", false).unwrap();
    assert!(diff_unstaged.contains("+new line"));

    // Unstage after staging
    stage_file(&repo, "test.txt").unwrap();
    unstage_file(&repo, "test.txt").unwrap();
    let diff_unstaged_after = get_file_diff(&repo, "test.txt", false).unwrap();
    assert!(diff_unstaged_after.contains("+new line"));
}

#[test]
fn test_branches_create_rename_delete() {
    let (dir, repo) = setup_temp_repo();

    // Create initial commit first (branches need a HEAD to start)
    let file_path = dir.path().join("test.txt");
    File::create(&file_path).unwrap();
    stage_file(&repo, "test.txt").unwrap();
    commit(&repo, "init", "Test User", "test@example.com").unwrap();

    // Create branch
    create_branch(&repo, "feature-1", "HEAD").unwrap();

    // List branches
    let branches = get_branches(&repo);
    assert!(branches.iter().any(|b| b.name == "feature-1"));

    // Rename
    rename_branch(&repo, "feature-1", "feature-2").unwrap();
    let branches = get_branches(&repo);
    assert!(!branches.iter().any(|b| b.name == "feature-1"));
    assert!(branches.iter().any(|b| b.name == "feature-2"));

    // Delete
    delete_branch(&repo, "feature-2").unwrap();
    let branches = get_branches(&repo);
    assert!(!branches.iter().any(|b| b.name == "feature-2"));
}

#[test]
fn test_stash() {
    let (dir, mut repo) = setup_temp_repo();
    let file_path = dir.path().join("test.txt");
    File::create(&file_path).unwrap();
    stage_file(&repo, "test.txt").unwrap();
    commit(&repo, "init", "Test User", "test@example.com").unwrap();

    // Make modifications
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "modified").unwrap();
    }

    // Stash
    stash_save(&mut repo, Some("my stash"), false).unwrap();

    // List stash
    let stashes = list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("my stash"));

    // Apply stash
    stash_apply(&mut repo, 0).unwrap();

    // Drop stash
    stash_drop(&mut repo, 0).unwrap();
    let stashes_after = list_stashes(&mut repo).unwrap();
    assert_eq!(stashes_after.len(), 0);
}

#[test]
fn test_reset_and_merge() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("test.txt");
    File::create(&file_path).unwrap();
    stage_file(&repo, "test.txt").unwrap();
    let first_oid = commit(&repo, "init", "Test User", "test@example.com").unwrap();

    // Create second commit
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "version 2").unwrap();
    }
    stage_file(&repo, "test.txt").unwrap();
    let second_oid = commit(&repo, "v2", "Test User", "test@example.com").unwrap();

    // Create branch at first commit
    create_branch(&repo, "other", &first_oid.to_string()).unwrap();

    // Reset to first commit
    reset(&repo, &first_oid.to_string(), ResetMode::Hard).unwrap();
    assert_eq!(
        repo.head().unwrap().peel_to_commit().unwrap().id(),
        first_oid
    );

    // Merge branch 'v2-branch' (fast-forward)
    create_branch(&repo, "v2-branch", &second_oid.to_string()).unwrap();
    let analysis = merge(&repo, "v2-branch").unwrap();
    assert!(analysis.is_fast_forward());
    assert_eq!(
        repo.head().unwrap().peel_to_commit().unwrap().id(),
        second_oid
    );
}

#[test]
fn test_checkout_local_and_remote_branch() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "initial commit").unwrap();
    }
    stage_file(&repo, "file.txt").unwrap();
    let init_oid = commit(&repo, "initial", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    // 1. Local branch creation and checkout
    create_branch(&repo, "feature-local", "HEAD").unwrap();
    let checked_out = checkout_branch(&repo, "feature-local").unwrap();
    assert_eq!(checked_out, "feature-local");
    assert_eq!(repo.head().unwrap().shorthand().ok(), Some("feature-local"));

    // 2. Switch back to default branch
    checkout_branch(&repo, &default_branch).unwrap();
    assert_eq!(
        repo.head().unwrap().shorthand().ok(),
        Some(default_branch.as_str())
    );

    // 3. Create a second commit for remote branch
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "remote commit content").unwrap();
    }
    stage_file(&repo, "file.txt").unwrap();
    let remote_oid = commit(&repo, "remote feat", "Test User", "test@example.com").unwrap();

    // Reset local main back to initial commit so HEAD doesn't have the new commit
    reset(&repo, &init_oid.to_string(), ResetMode::Hard).unwrap();

    // Setup a fake remote and remote references
    repo.remote("origin", "https://example.com/repo.git")
        .unwrap();
    repo.reference(
        "refs/remotes/origin/feature-remote",
        remote_oid,
        true,
        "create remote tracking ref",
    )
    .unwrap();
    // Also create a symbolic HEAD ref to test that it is excluded from branch list
    repo.reference_symbolic(
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
        true,
        "create remote HEAD",
    )
    .unwrap();

    // Verify get_branches filters out origin/HEAD and lists origin/feature-remote
    let branches = get_branches(&repo);
    assert!(!branches.iter().any(|b| b.name.ends_with("/HEAD")));
    let remote_entry = branches
        .iter()
        .find(|b| b.name == "origin/feature-remote")
        .expect("origin/feature-remote must be in branch list");
    assert!(remote_entry.is_remote);

    // 4. Checkout the remote branch: should create local "feature-remote", track origin, and switch
    let checked_out_remote = checkout_branch(&repo, "origin/feature-remote").unwrap();
    assert_eq!(checked_out_remote, "feature-remote");
    assert_eq!(
        repo.head().unwrap().shorthand().ok(),
        Some("feature-remote")
    );

    // Verify local branch was created and points to remote_oid
    let local_branch = repo
        .find_branch("feature-remote", git2::BranchType::Local)
        .expect("local branch should exist");
    assert_eq!(
        local_branch.get().peel_to_commit().unwrap().id(),
        remote_oid
    );

    // Verify upstream tracking configuration
    let upstream = local_branch
        .upstream()
        .expect("upstream should be configured");
    assert_eq!(upstream.name().unwrap(), Some("origin/feature-remote"));

    // Verify working tree contains remote commit content
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("remote commit content"));

    // 5. Checkout again when local branch already exists: should succeed and stay on it
    let checked_out_again = checkout_branch(&repo, "origin/feature-remote").unwrap();
    assert_eq!(checked_out_again, "feature-remote");

    // 6. Test merge of a remote branch into default branch
    checkout_branch(&repo, &default_branch).unwrap();
    let merge_analysis = merge(&repo, "origin/feature-remote").unwrap();
    assert!(merge_analysis.is_fast_forward());
    assert_eq!(
        repo.head().unwrap().peel_to_commit().unwrap().id(),
        remote_oid
    );
}

#[test]
fn test_stage_all_and_unstage_all() {
    let (dir, repo) = setup_temp_repo();
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");

    File::create(&file1)
        .unwrap()
        .write_all(b"content 1")
        .unwrap();
    File::create(&file2)
        .unwrap()
        .write_all(b"content 2")
        .unwrap();

    let status = get_status(&repo);
    assert_eq!(status.len(), 2);
    assert!(!status[0].is_staged);
    assert!(!status[1].is_staged);

    // Stage all
    stage_all(&repo).unwrap();
    let status = get_status(&repo);
    assert_eq!(status.len(), 2);
    assert!(status[0].is_staged);
    assert!(status[1].is_staged);

    // Unstage all
    unstage_all(&repo).unwrap();
    let status = get_status(&repo);
    assert_eq!(status.len(), 2);
    assert!(!status[0].is_staged);
    assert!(!status[1].is_staged);
}

#[test]
fn test_discard_file_changes() {
    let (dir, repo) = setup_temp_repo();
    let tracked_path = dir.path().join("tracked.txt");
    let untracked_path = dir.path().join("untracked.txt");

    // Commit initial tracked file
    File::create(&tracked_path)
        .unwrap()
        .write_all(b"v1 content")
        .unwrap();
    stage_file(&repo, "tracked.txt").unwrap();
    commit(&repo, "initial", "Test User", "test@example.com").unwrap();

    // Modify tracked file and create untracked file
    File::create(&tracked_path)
        .unwrap()
        .write_all(b"v2 modified")
        .unwrap();
    File::create(&untracked_path)
        .unwrap()
        .write_all(b"untracked content")
        .unwrap();

    // Stage tracked file
    stage_file(&repo, "tracked.txt").unwrap();

    // Discard tracked changes
    discard_file_changes(&repo, "tracked.txt").unwrap();
    let restored = std::fs::read_to_string(&tracked_path).unwrap();
    assert_eq!(restored, "v1 content");
    let status = get_status(&repo);
    assert!(!status.iter().any(|s| s.path == "tracked.txt"));

    // Discard untracked file (should delete file)
    assert!(untracked_path.exists());
    discard_file_changes(&repo, "untracked.txt").unwrap();
    assert!(!untracked_path.exists());
}

#[test]
fn test_commit_amend() {
    let (dir, repo) = setup_temp_repo();
    let file_path = dir.path().join("file.txt");

    File::create(&file_path)
        .unwrap()
        .write_all(b"version 1")
        .unwrap();
    stage_file(&repo, "file.txt").unwrap();
    let init_oid = commit(&repo, "Initial commit", "Test User", "test@example.com").unwrap();

    // Modify file and stage
    File::create(&file_path)
        .unwrap()
        .write_all(b"version 2")
        .unwrap();
    stage_file(&repo, "file.txt").unwrap();

    let amended_oid =
        commit_amend(&repo, "Amended commit", "Test User", "test@example.com").unwrap();
    assert_ne!(init_oid, amended_oid);

    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head_commit.id(), amended_oid);
    assert_eq!(head_commit.message().unwrap(), "Amended commit");
    // Should still have no parents since init was root
    assert_eq!(head_commit.parent_count(), 0);
}

#[test]
fn test_add_to_gitignore() {
    let (dir, repo) = setup_temp_repo();
    add_to_gitignore(&repo, "*.log").unwrap();
    add_to_gitignore(&repo, "target/").unwrap();

    let gitignore_path = dir.path().join(".gitignore");
    let content = std::fs::read_to_string(gitignore_path).unwrap();
    assert!(content.contains("*.log"));
    assert!(content.contains("target/"));
}

#[test]
fn test_abort_merge() {
    let (dir, repo) = setup_temp_repo();
    let conflict_file = dir.path().join("conflict.txt");

    File::create(&conflict_file)
        .unwrap()
        .write_all(b"base line\n")
        .unwrap();
    stage_file(&repo, "conflict.txt").unwrap();
    commit(&repo, "base", "Test User", "test@example.com").unwrap();

    let default_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    create_branch(&repo, "feature", "HEAD").unwrap();

    // Modify on default branch
    File::create(&conflict_file)
        .unwrap()
        .write_all(b"modified on default\n")
        .unwrap();
    stage_file(&repo, "conflict.txt").unwrap();
    commit(&repo, "change default", "Test User", "test@example.com").unwrap();

    // Switch to feature and modify differently
    checkout_branch(&repo, "feature").unwrap();
    File::create(&conflict_file)
        .unwrap()
        .write_all(b"modified on feature\n")
        .unwrap();
    stage_file(&repo, "conflict.txt").unwrap();
    commit(&repo, "change feature", "Test User", "test@example.com").unwrap();

    // Switch back to default and merge feature
    checkout_branch(&repo, &default_branch).unwrap();
    let analysis = merge(&repo, "feature").unwrap();
    assert!(analysis.is_normal());
    assert_eq!(repo.state(), git2::RepositoryState::Merge);

    // Abort merge
    abort_merge(&repo).unwrap();
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    let content = std::fs::read_to_string(&conflict_file).unwrap();
    assert_eq!(content.trim(), "modified on default");
}

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
    push(&repo, "test-remote", &default_branch, true).unwrap();

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
    push(&repo, "test-remote", "feat-to-delete", true).unwrap();

    // Remote bare repo must have the branch
    assert!(
        remote_repo
            .find_branch("feat-to-delete", git2::BranchType::Local)
            .is_ok()
    );

    // Delete remote branch
    delete_remote_branch(&repo, "test-remote", "feat-to-delete").unwrap();

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
    push(&repo, "origin", &default_branch, true).unwrap();

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
    push(&repo, "origin", &default_branch, true).unwrap();

    create_tag(&repo, "v1.5.0", "HEAD", None).unwrap();
    push_tags(&repo, "origin").unwrap();

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

    let cloned_repo = clone_repo(&src_url, &clone_target_path).unwrap();
    assert!(clone_target_path.join("cloned_file.txt").exists());

    let content = std::fs::read_to_string(clone_target_path.join("cloned_file.txt")).unwrap();
    assert!(content.contains("hello clone"));

    let cloned_head = cloned_repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(cloned_head.id(), src_oid);
}

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
