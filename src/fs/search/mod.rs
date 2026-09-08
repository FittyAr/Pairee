pub mod scanner;
pub mod types;

pub use scanner::find_files;
pub use types::{SearchQuery, SearchTarget};

#[cfg(test)]
mod tests {
    use super::scanner::MAX_SEARCH_DEPTH;
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

    #[tokio::test]
    async fn test_find_files_respects_depth_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
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
