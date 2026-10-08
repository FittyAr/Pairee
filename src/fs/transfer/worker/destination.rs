use std::path::PathBuf;

/// Determines if `destination` should be treated as a parent directory into which
/// source items are placed (appending source item filenames), or if `destination` is
/// the target path for a single source item itself.
pub fn is_destination_parent_dir(
    sources: &[PathBuf],
    destination: &std::path::Path,
    is_dir_fn: impl FnOnce(&std::path::Path) -> bool,
) -> bool {
    if sources.len() > 1 {
        return true;
    }
    let s = destination.to_string_lossy();
    if s.ends_with('/') || s.ends_with('\\') {
        return true;
    }
    if is_dir_fn(destination) {
        if let Some(src) = sources.first()
            && let (Some(dest_name), Some(src_name)) = (destination.file_name(), src.file_name())
        {
            return dest_name != src_name;
        }
        return true;
    }
    false
}

/// Creates `dir` with its missing parents, appending every folder it had to
/// create (outermost first) to `created`.
pub(super) fn ensure_dir(dir: &std::path::Path, created: &mut Vec<PathBuf>) {
    let missing: Vec<PathBuf> = dir
        .ancestors()
        .take_while(|p| !p.as_os_str().is_empty() && !p.exists())
        .map(std::path::Path::to_path_buf)
        .collect();
    if missing.is_empty() || std::fs::create_dir_all(dir).is_err() {
        return;
    }
    created.extend(missing.into_iter().rev().filter(|p| p.is_dir()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_dir_reports_only_new_folders() {
        let root = tempfile::tempdir().unwrap();
        let mut created = Vec::new();
        ensure_dir(root.path(), &mut created);
        assert!(created.is_empty());
        let deep = root.path().join("a").join("b");
        ensure_dir(&deep, &mut created);
        assert_eq!(created, vec![root.path().join("a"), deep.clone()]);
        ensure_dir(&deep, &mut created);
        assert_eq!(created.len(), 2);
    }

    #[test]
    fn test_is_destination_parent_dir_single_file_target_path() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs/reporte.md");
        assert!(!is_destination_parent_dir(&sources, &destination, |_| {
            false
        }));
    }

    #[test]
    fn test_is_destination_parent_dir_trailing_slash() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs/");
        assert!(is_destination_parent_dir(&sources, &destination, |_| false));
    }

    #[test]
    fn test_is_destination_parent_dir_existing_folder_different_name() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs");
        assert!(is_destination_parent_dir(&sources, &destination, |_| true));
    }

    #[test]
    fn test_is_destination_parent_dir_multiple_sources() {
        let sources = vec![
            PathBuf::from("/home/user/file1.md"),
            PathBuf::from("/home/user/file2.md"),
        ];
        let destination = PathBuf::from("/home/user/docs/file1.md");
        assert!(is_destination_parent_dir(&sources, &destination, |_| false));
    }
}
