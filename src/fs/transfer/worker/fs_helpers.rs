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

pub(super) fn make_writable_helper(path: &std::path::Path) -> std::io::Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let mut perms = metadata.permissions();
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.mode();
        let is_dir = metadata.is_dir();
        let new_mode = if is_dir { mode | 0o700 } else { mode | 0o600 };
        perms.set_mode(new_mode);
    }
    #[cfg(target_os = "windows")]
    {
        // Windows: clear readonly bit before delete
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
    }
    std::fs::set_permissions(path, perms)
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
