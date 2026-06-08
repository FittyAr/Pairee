use std::path::PathBuf;
use tokio::sync::mpsc;

/// Search query parameters.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Glob name pattern (e.g. "*.rs"). Empty string means match all.
    pub name_glob: String,
    /// Optional text to search inside file content. None = skip content search.
    pub content: Option<String>,
    /// Root directory to start the recursive search.
    pub root: PathBuf,
}

/// Spawns a background Tokio task that searches for files matching `query`.
/// Matching paths are sent through the returned `Receiver<PathBuf>`.
/// The channel closes when the search is complete.
pub fn find_files(query: SearchQuery) -> mpsc::Receiver<PathBuf> {
    let (tx, rx) = mpsc::channel(256);

    tokio::spawn(async move {
        search_recursive(&query.root, &query, &tx).await;
    });

    rx
}

/// Recursive async search through a directory tree.
async fn search_recursive(dir: &PathBuf, query: &SearchQuery, tx: &mpsc::Sender<PathBuf>) {
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
            if file_type.is_dir() {
                // Recurse into subdirectory (Box::pin to avoid infinite type recursion)
                Box::pin(search_recursive(&path, query, tx)).await;
            } else if file_type.is_file() {
                // 1. Check name pattern
                let name_matches = query.name_glob.is_empty()
                    || crate::app::state::glob_matches(&query.name_glob, &name);

                if !name_matches {
                    continue;
                }

                // 2. Optionally check file content
                let content_matches = match &query.content {
                    None => true,
                    Some(needle) => {
                        file_contains(path.as_path(), needle).await
                    }
                };

                if content_matches {
                    // Channel send — if receiver is dropped, abort search
                    if tx.send(path).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

/// Returns true if the text file at `path` contains `needle` (case-insensitive).
/// Non-UTF-8 / binary files return false.
async fn file_contains(path: &std::path::Path, needle: &str) -> bool {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => content.to_lowercase().contains(&needle.to_lowercase()),
        Err(_) => false, // Binary or unreadable file — skip
    }
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
        };

        let mut rx = find_files(query);
        let mut found = Vec::new();
        while let Some(path) = rx.recv().await {
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
        };

        let mut rx = find_files(query);
        let mut found = Vec::new();
        while let Some(path) = rx.recv().await {
            found.push(path);
        }
        assert_eq!(found.len(), 1);
        assert!(found[0].file_name().unwrap() == "a.txt");
    }
}
