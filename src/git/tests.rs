#[cfg(test)]
mod tests {
    use crate::git::branches::*;
    use crate::git::commit::*;
    use crate::git::diff::*;
    use crate::git::merge::*;
    use crate::git::repo::*;
    use crate::git::reset::*;
    use crate::git::stage::*;
    use crate::git::stash::*;
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
}
