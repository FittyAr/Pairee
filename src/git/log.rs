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
