use std::str;

/// Returns the unified diff of a specific file.
/// If `is_staged` is true, returns the diff between HEAD and the index (staged changes).
/// If `is_staged` is false, returns the diff between the index and the working directory (unstaged changes).
pub fn get_file_diff(repo: &git2::Repository, file_path: &str, is_staged: bool) -> anyhow::Result<String> {
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = if is_staged {
        let head_tree = if let Ok(head_ref) = repo.head() {
            let commit = head_ref.peel_to_commit()?;
            Some(commit.tree()?)
        } else {
            None
        };
        let index = repo.index()?;
        repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), Some(&mut opts))?
    } else {
        let index = repo.index()?;
        repo.diff_index_to_workdir(Some(&index), Some(&mut opts))?
    };

    diff_to_string(&diff)
}

/// Returns the unified diff showing changes introduced by a specific commit.
pub fn get_commit_diff(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<String> {
    let oid = git2::Oid::from_str(commit_hash)?;
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0)?;
        Some(parent.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    diff_to_string(&diff)
}

/// Helper function to convert a git2::Diff object to a unified patch string.
fn diff_to_string(diff: &git2::Diff) -> anyhow::Result<String> {
    let mut out = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        match origin {
            '+' | '-' | ' ' => {
                out.push(origin);
            }
            _ => {}
        }
        if let Ok(s) = str::from_utf8(line.content()) {
            out.push_str(s);
        }
        true
    })?;
    Ok(out)
}
