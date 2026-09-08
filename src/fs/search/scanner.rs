use super::types::{SearchQuery, SearchTarget};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Upper bound on the depth of the recursive directory walk.
pub const MAX_SEARCH_DEPTH: usize = 32;

/// Spawns a background Tokio task that searches for files and folders matching `query`.
pub fn find_files(query: SearchQuery) -> mpsc::Receiver<(PathBuf, bool)> {
    let (tx, rx) = mpsc::channel(256);

    tokio::spawn(async move {
        search_recursive(&query.root, &query, &tx, 0).await;
    });

    rx
}

/// Recursive async search through a directory tree.
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
                Box::pin(search_recursive(&path, query, tx, depth + 1)).await;

                if query.target == SearchTarget::Any || query.target == SearchTarget::Directory {
                    let name_matches = query.name_glob.is_empty()
                        || crate::app::state::glob_matches_case(
                            &query.name_glob,
                            &name,
                            query.case_sensitive,
                        );
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

/// Returns true if the text file at `path` contains `needle` (case-insensitive unless configured).
async fn file_contains(path: &std::path::Path, needle: &str, case_sensitive: bool) -> bool {
    const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
    let size_ok = match tokio::fs::metadata(path).await {
        Ok(m) => m.len() <= MAX_FILE_BYTES,
        Err(_) => false,
    };
    if !size_ok {
        return false;
    }

    use tokio::io::{AsyncBufReadExt, BufReader};
    let Ok(file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let reader = BufReader::with_capacity(64 * 1024, file);
    let mut lines = reader.lines();
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
            let lower = line.to_lowercase();
            if let Some(ref n) = needle_lc
                && lower.contains(n)
            {
                return true;
            }
        }
    }
    false
}
