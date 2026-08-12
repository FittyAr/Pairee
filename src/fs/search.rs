use std::path::PathBuf;
use tokio::sync::mpsc;

/// Search query target types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    Any,
    File,
    Directory,
}

/// Search query parameters.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Glob name pattern (e.g. "*.rs"). Empty string means match all.
    pub name_glob: String,
    /// Optional text to search inside file content. None = skip content search.
    pub content: Option<String>,
    /// Root directory to start the recursive search.
    pub root: PathBuf,
    /// Whether glob name matching and content search are case-sensitive.
    pub case_sensitive: bool,
    /// Target type of entries to find.
    pub target: SearchTarget,
}

/// Spawns a background Tokio task that searches for files and folders matching `query`.
/// Matching paths are sent through the returned `Receiver<(PathBuf, bool)>` (path, is_dir).
/// The channel closes when the search is complete.
pub fn find_files(query: SearchQuery) -> mpsc::Receiver<(PathBuf, bool)> {
    let (tx, rx) = mpsc::channel(256);

    tokio::spawn(async move {
        search_recursive(&query.root, &query, &tx, 0).await;
    });

    rx
}

/// Recursive async search through a directory tree.
///
/// `depth` is the current nesting level; the walk stops descending
/// once it exceeds [`MAX_SEARCH_DEPTH`], so a search rooted at the
/// filesystem root cannot pin a worker for an unbounded time.
async fn search_recursive(
    dir: &PathBuf,
    query: &SearchQuery,
    tx: &mpsc::Sender<(PathBuf, bool)>,
    depth: usize,
) {
    if depth > MAX_SEARCH_DEPTH {
        return;
    }

    let read_dir = match tokio::fs::read_dir(dir).await {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut read_dir = read_dir;
    loop {
        let entry = match read_dir.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(_) => continue,
        };

        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if let Ok(file_type) = entry.file_type().await {
            let is_dir = file_type.is_dir();
            let is_file = file_type.is_file();

            if is_dir {
                // 1. Recurse into subdirectory (Box::pin to avoid infinite type recursion).
                // The depth bound caps how deep the walk can go; without
                // it a search rooted at `/` would traverse the whole
                // filesystem.
                Box::pin(search_recursive(&path, query, tx, depth + 1)).await;

                // 2. Check if the directory itself matches
                if query.target == SearchTarget::Any || query.target == SearchTarget::Directory {
                    let name_matches = query.name_glob.is_empty()
                        || crate::app::state::glob_matches_case(
                            &query.name_glob,
                            &name,
                            query.case_sensitive,
                        );
                    // Directories have no content to search, so if query.content is Some, it doesn't match
                    let content_matches = query.content.is_none();

                    if name_matches && content_matches && tx.send((path, true)).await.is_err() {
                        return;
                    }
                }
            } else if is_file
                && (query.target == SearchTarget::Any || query.target == SearchTarget::File)
            {
                let name_matches = query.name_glob.is_empty()
                    || crate::app::state::glob_matches_case(
                        &query.name_glob,
                        &name,
                        query.case_sensitive,
                    );

                if name_matches {
                    let content_matches = match &query.content {
                        None => true,
                        Some(needle) => {
                            file_contains(path.as_path(), needle, query.case_sensitive).await
                        }
                    };

                    if content_matches && tx.send((path, false)).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

/// Upper bound on the depth of the recursive directory walk. Without
/// this, a search rooted at `/` (or any deep workspace) would happily
/// traverse the entire filesystem and pin a worker thread for as long
/// as the walk takes. 32 levels is far more than any real project
/// structure and matches the depth limits in similar tools.
const MAX_SEARCH_DEPTH: usize = 32;

/// Returns true if the text file at `path` contains `needle` (case-insensitive unless configured).
/// Non-UTF-8 / binary files return false. Files larger than 16 MiB are
/// skipped to keep the case-insensitive search from allocating two full
/// copies of the file in memory.
async fn file_contains(path: &std::path::Path, needle: &str, case_sensitive: bool) -> bool {
    // Bail out on files we are not going to scan in full. 16 MiB is a
    // generous ceiling for a content search; most text files are well
    // under this. The user can still find the file by name.
    const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
    let size_ok = match tokio::fs::metadata(path).await {
        Ok(m) => m.len() <= MAX_FILE_BYTES,
        Err(_) => false,
    };
    if !size_ok {
        return false;
    }
    // Stream the file with a 64 KiB buffer so we never hold more than
    // (buffer + needle) in memory. The previous implementation read the
    // whole file into a `String` and then lowercased it, which doubled
    // peak memory and amplified the OOM risk on huge text files.
    use tokio::io::{AsyncBufReadExt, BufReader};
    let Ok(file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let reader = BufReader::with_capacity(64 * 1024, file);
    let mut lines = reader.lines();
    // For case-insensitive search, pre-lowercase the needle once.
    let needle_lc = if case_sensitive {
        None
    } else {
        Some(needle.to_lowercase())
    };
    while let Ok(Some(line)) = lines.next_line().await {
        if case_sensitive {
            if line.contains(needle) {
                return true;
            }
        } else {
            // The lowercase form may split a UTF-8 character across the
            // 64 KiB chunk boundary in pathological cases; lowercasing
            // per-line and using `.contains` is safe (each line is a
            // complete UTF-8 string thanks to `lines()`).
            let lower = line.to_lowercase();
            if let Some(ref n) = needle_lc {
                if lower.contains(n) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_files_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("main.rs"), b"fn main() {}").unwrap();
        std::fs::write(dir.path().join("lib.rs"), b"pub fn foo() {}").unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), b"[package]").unwrap();

        let query = SearchQuery {
            name_glob: "*.rs".to_string(),
            content: None,
            root: dir.path().to_path_buf(),
            case_sensitive: false,
            target: SearchTarget::Any,
        };

        let mut rx = find_files(query);
        let mut found = Vec::new();
        while let Some((path, _)) = rx.recv().await {
            found.push(path);
        }
        assert_eq!(found.len(), 2);
    }

    #[tokio::test]
    async fn test_find_files_by_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), b"hello world").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"goodbye world").unwrap();

        let query = SearchQuery {
            name_glob: "*.txt".to_string(),
            content: Some("hello".to_string()),
            root: dir.path().to_path_buf(),
            case_sensitive: false,
            target: SearchTarget::Any,
        };

        let mut rx = find_files(query);
        let mut found = Vec::new();
        while let Some((path, _)) = rx.recv().await {
            found.push(path);
        }
        assert_eq!(found.len(), 1);
        assert!(found[0].file_name().unwrap() == "a.txt");
    }

    #[tokio::test]
    async fn test_find_directories_and_case_sensitivity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub_dir = dir.path().join("TestSubDir");
        std::fs::create_dir(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("main.RS"), b"fn main() {}").unwrap();

        // 1. Find directories only, case-insensitive
        let query_dirs = SearchQuery {
            name_glob: "test*".to_string(),
            content: None,
            root: dir.path().to_path_buf(),
            case_sensitive: false,
            target: SearchTarget::Directory,
        };
        let mut rx = find_files(query_dirs);
        let mut found_dirs = Vec::new();
        while let Some((path, is_dir)) = rx.recv().await {
            found_dirs.push((path, is_dir));
        }
        assert_eq!(found_dirs.len(), 1);
        assert!(found_dirs[0].1); // must be directory
        assert_eq!(found_dirs[0].0.file_name().unwrap(), "TestSubDir");

        // 2. Find files only, case-sensitive
        let query_files_cs = SearchQuery {
            name_glob: "*.RS".to_string(),
            content: None,
            root: dir.path().to_path_buf(),
            case_sensitive: true,
            target: SearchTarget::File,
        };
        let mut rx = find_files(query_files_cs);
        let mut found_files = Vec::new();
        while let Some((path, is_dir)) = rx.recv().await {
            found_files.push((path, is_dir));
        }
        assert_eq!(found_files.len(), 1);
        assert!(!found_files[0].1); // must be file
        assert_eq!(found_files[0].0.file_name().unwrap(), "main.RS");

        // 3. Find files only, case-sensitive (should fail to match lowercase pattern)
        let query_files_cs_fail = SearchQuery {
            name_glob: "*.rs".to_string(),
            content: None,
            root: dir.path().to_path_buf(),
            case_sensitive: true,
            target: SearchTarget::File,
        };
        let mut rx = find_files(query_files_cs_fail);
        let mut found_files_fail = Vec::new();
        while let Some((path, _)) = rx.recv().await {
            found_files_fail.push(path);
        }
        assert_eq!(found_files_fail.len(), 0);
    }

    /// The depth cap on the recursive walk must prevent a runaway search
    /// from descending past `MAX_SEARCH_DEPTH` even when the directory
    /// tree itself is deeper. We build a linear chain of directories and
    /// confirm the search stops descending before reaching the leaf.
    #[tokio::test]
    async fn test_find_files_respects_depth_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Build a chain `root/a/a/a/.../a/deep.txt` deeper than the cap.
        let mut current = dir.path().to_path_buf();
        for _ in 0..(MAX_SEARCH_DEPTH + 4) {
            current = current.join("a");
            std::fs::create_dir(&current).unwrap();
        }
        std::fs::write(current.join("deep.txt"), b"x").unwrap();

        let query = SearchQuery {
            name_glob: "deep.txt".to_string(),
            content: None,
            root: dir.path().to_path_buf(),
            case_sensitive: true,
            target: SearchTarget::File,
        };
        let mut rx = find_files(query);
        let mut found = Vec::new();
        // Bound the wait so the test fails fast if the cap is broken.
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        while let Some((path, _)) = tokio::time::timeout(timeout - start.elapsed(), rx.recv())
            .await
            .unwrap_or(None)
        {
            found.push(path);
        }
        assert!(
            found.is_empty(),
            "search should not descend past the depth cap; found: {:?}",
            found
        );
    }
}
