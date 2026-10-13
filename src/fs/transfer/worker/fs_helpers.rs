/// Moves `path` to the platform trash (Recycle Bin, Finder Trash, XDG trash).
///
/// Never falls back to a permanent delete: if the trash is unavailable the
/// error is reported and the item is left in place, so the user can decide to
/// delete it permanently instead.
pub(super) fn send_to_recycle_bin_helper(path: &std::path::Path) -> anyhow::Result<()> {
    trash::delete(path).map_err(|e| {
        anyhow::anyhow!(
            crate::config::localization::t("error_trash_failed")
                .replacen("{}", &path.to_string_lossy(), 1)
                .replacen("{}", &e.to_string(), 1)
        )
    })
}

/// Puts the trashed files that were at `paths` back (the undo of a copy
/// that replaced them); a path that is occupied again is left alone.
pub(super) fn restore_from_trash(
    paths: &[std::path::PathBuf],
    ctl: &crate::fs::transfer::control::JobControl,
    results: &mut crate::fs::transfer::job::TransferResults,
) {
    let pending: Vec<_> = paths
        .iter()
        .filter(|p| !crate::fs::stamp::entry_exists(p))
        .collect();
    if pending.is_empty() {
        return;
    }
    let mut index = match crate::fs::journal::TrashIndex::load() {
        Ok(index) => index,
        Err(error) => {
            for path in pending {
                ctl.file_failed(results, failed(path, error.clone()));
            }
            return;
        }
    };
    for path in pending {
        if let Err(error) = index.restore(path) {
            ctl.file_failed(results, failed(path, error));
        }
    }
}

fn failed(path: &std::path::Path, error: String) -> crate::fs::transfer::job::FailedFile {
    crate::fs::transfer::control::failed(path, std::path::PathBuf::new(), error)
}

#[cfg(test)]
mod tests {
    use super::send_to_recycle_bin_helper;

    #[test]
    fn missing_path_is_an_error_and_never_deletes_anything() {
        let dir = tempfile::tempdir().unwrap();
        let keep = dir.path().join("keep.txt");
        std::fs::write(&keep, b"x").unwrap();
        assert!(send_to_recycle_bin_helper(&dir.path().join("missing")).is_err());
        assert!(keep.exists());
    }
}
