use crate::config::localization::t;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::events::TransferEvent;
use crate::fs::transfer::job::DestinationKind;
use std::path::{Path, PathBuf};

/// Determines if `destination` should be treated as a parent directory into which
/// source items are placed (appending source item filenames), or if `destination` is
/// the target path for a single source item itself.
///
/// A `kind` of [`DestinationKind::Folder`] settles it: the path is the parent
/// even when it has the name of the single source. Otherwise an existing
/// folder named like the source is its target (the dialog pre-fills
/// `<folder>/<name>`), so copying onto it merges.
pub fn is_destination_parent_dir(
    sources: &[PathBuf],
    destination: &std::path::Path,
    kind: DestinationKind,
    is_dir_fn: impl FnOnce(&std::path::Path) -> bool,
) -> bool {
    if kind == DestinationKind::Folder || sources.len() > 1 {
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
pub(crate) fn ensure_dir(dir: &std::path::Path, created: &mut Vec<PathBuf>) {
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

/// Warns (without stopping the job) when the volume of `destination` has
/// less free space than the job needs. The destination may not exist yet:
/// its nearest existing folder is measured.
pub(super) fn warn_low_space(destination: &Path, needed: u64, ctl: &JobControl) {
    let Some(existing) = destination
        .ancestors()
        .find(|p| !p.as_os_str().is_empty() && p.exists())
    else {
        return;
    };
    let Ok(free) = crate::fs::transfer::network::get_free_space(existing) else {
        return;
    };
    if free >= needed {
        return;
    }
    ctl.emit(TransferEvent::Warning {
        job_id: ctl.job_id,
        message: t("transfer_low_space_warning")
            .replacen("{}", &bytesize::ByteSize(needed).to_string(), 1)
            .replacen("{}", &bytesize::ByteSize(free).to_string(), 1),
    });
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
        assert!(!is_destination_parent_dir(
            &sources,
            &destination,
            DestinationKind::Inferred,
            |_| false
        ));
    }

    #[test]
    fn test_is_destination_parent_dir_trailing_slash() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs/");
        assert!(is_destination_parent_dir(
            &sources,
            &destination,
            DestinationKind::Inferred,
            |_| false
        ));
    }

    #[test]
    fn test_is_destination_parent_dir_existing_folder_different_name() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs");
        assert!(is_destination_parent_dir(
            &sources,
            &destination,
            DestinationKind::Inferred,
            |_| true
        ));
    }

    /// The panel at `/backup/project` receives `/a/project`: told it is a
    /// folder, the item nests; inferred, the same path is the merge target.
    #[test]
    fn a_folder_named_like_the_source_nests_when_it_is_the_destination_folder() {
        let sources = vec![PathBuf::from("/a/project")];
        let destination = PathBuf::from("/backup/project");
        let check = |kind| is_destination_parent_dir(&sources, &destination, kind, |_| true);
        assert!(!check(DestinationKind::Inferred));
        assert!(check(DestinationKind::Folder));
    }

    #[test]
    fn test_is_destination_parent_dir_multiple_sources() {
        let sources = vec![
            PathBuf::from("/home/user/file1.md"),
            PathBuf::from("/home/user/file2.md"),
        ];
        let destination = PathBuf::from("/home/user/docs/file1.md");
        assert!(is_destination_parent_dir(
            &sources,
            &destination,
            DestinationKind::Inferred,
            |_| false
        ));
    }
}
