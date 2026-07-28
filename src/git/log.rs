/// A single commit entry for display in the log.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Short 7-character hash
    pub hash_short: String,
    /// Full 40-character hash (used for checkout)
    pub hash_full: String,
    /// Author display name
    pub author: String,
    /// ISO date string (YYYY-MM-DD)
    pub date: String,
    /// First line of the commit message
    pub message: String,
}

/// Reads up to `limit` commits from the HEAD of the active branch.
pub fn get_log(repo: &git2::Repository, limit: usize) -> Vec<CommitInfo> {
    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    if revwalk.push_head().is_err() {
        return Vec::new();
    }

    revwalk
        .filter_map(|oid_res| {
            let oid = oid_res.ok()?;
            let commit = repo.find_commit(oid).ok()?;

            let hash_full = oid.to_string();
            let hash_short = hash_full[..7.min(hash_full.len())].to_string();

            let author = commit.author().name().unwrap_or("unknown").to_string();

            let timestamp = commit.author().when().seconds();
            let naive = chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "????-??-??".to_string());

            let message = commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();

            Some(CommitInfo {
                hash_short,
                hash_full,
                author,
                date: naive,
                message,
            })
        })
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::commit::commit;
    use crate::git::repo::init_repo;
    use crate::git::stage::stage_file;
    use std::fs::File;
    use tempfile::TempDir;

    fn setup_with_commits(n: usize) -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        let mut c = repo.config().unwrap();
        c.set_str("user.name", "T").unwrap();
        c.set_str("user.email", "t@t").unwrap();
        for i in 0..n {
            let f = dir.path().join(format!("f{i}.txt"));
            File::create(&f).unwrap();
            stage_file(&repo, &format!("f{i}.txt")).unwrap();
            commit(&repo, &format!("commit {i}"), "T", "t@t").unwrap();
        }
        (dir, repo)
    }

    #[test]
    fn get_log_returns_empty_for_fresh_repo() {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        assert!(get_log(&repo, 10).is_empty());
    }

    #[test]
    fn get_log_returns_all_commits_up_to_limit() {
        let (_dir, repo) = setup_with_commits(3);
        let log = get_log(&repo, 10);
        assert_eq!(log.len(), 3);
        // Newest first.
        assert_eq!(log[0].message, "commit 2");
        assert_eq!(log[1].message, "commit 1");
        assert_eq!(log[2].message, "commit 0");
    }

    #[test]
    fn get_log_respects_limit() {
        let (_dir, repo) = setup_with_commits(5);
        let log = get_log(&repo, 2);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn get_log_short_hash_is_seven_chars() {
        let (_dir, repo) = setup_with_commits(1);
        let log = get_log(&repo, 1);
        assert_eq!(log[0].hash_short.len(), 7);
        assert_eq!(log[0].hash_full.len(), 40);
        // The short hash is a prefix of the full hash.
        assert!(log[0].hash_full.starts_with(&log[0].hash_short));
    }

    #[test]
    fn commit_info_carries_author_and_date() {
        let (_dir, repo) = setup_with_commits(1);
        let log = get_log(&repo, 1);
        // Author comes from the git config we set in setup.
        assert_eq!(log[0].author, "T");
        // Date is `YYYY-MM-DD` (10 chars, hyphens at 4 and 7).
        assert_eq!(log[0].date.len(), 10);
        assert_eq!(log[0].date.as_bytes()[4], b'-');
        assert_eq!(log[0].date.as_bytes()[7], b'-');
    }
}
