use crate::config::localization::t;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) fn delete_recursive(path: &Path) -> Result<()> {
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        fs::remove_file(path).map_err(|e| {
            anyhow::anyhow!(
                t("error_failed_remove_symlink")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1)
            )
        })
    } else if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| {
            anyhow::anyhow!(
                t("error_failed_delete_dir")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1)
            )
        })
    } else {
        fs::remove_file(path).map_err(|e| {
            anyhow::anyhow!(
                t("error_failed_delete_file")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1)
            )
        })
    }
}
