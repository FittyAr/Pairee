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

#[cfg(test)]
mod tests {
    use super::*;

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
