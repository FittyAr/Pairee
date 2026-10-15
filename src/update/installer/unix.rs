//! Linux and macOS: the binary in the release tarball replaces the running
//! one with an atomic rename (the running process keeps its old inode).

use anyhow::{Context as _, Result};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::{InstallResult, cleanup, extract};

/// Replaces the running binary with the one in the verified tarball.
pub(super) fn install(archive: &[u8]) -> Result<InstallResult> {
    let exe = live_exe_path(std::env::current_exe().context("cannot determine current exe path")?);
    let exe_dir = exe.parent().context("exe has no parent directory")?;

    let new_bin = extract::file_from_tar_gz(archive, "pairee")?;

    // Write to a randomly named sibling (created 0600 by tempfile), mark it
    // executable and atomically rename it over the running binary.
    let mut staged = tempfile::Builder::new()
        .prefix(cleanup::STAGING_PREFIX)
        .tempfile_in(exe_dir)
        .context("cannot create staging file next to pairee binary")?;
    staged
        .write_all(&new_bin)
        .context("failed to write new pairee binary")?;
    staged
        .as_file()
        .sync_all()
        .context("failed to flush new pairee binary")?;
    set_executable(staged.path())?;
    staged
        .persist(&exe)
        .map_err(|e| e.error)
        .context("failed to replace pairee binary")?;

    Ok(InstallResult::RestartRequired)
}

/// Linux reports a replaced binary as `<path> (deleted)`; the file to update
/// again is the one at `<path>`, which the previous update put there.
fn live_exe_path(exe: PathBuf) -> PathBuf {
    match exe.to_str().and_then(|s| s.strip_suffix(" (deleted)")) {
        Some(live) => PathBuf::from(live),
        None => exe,
    }
}

fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replaced_binary_maps_back_to_its_path() {
        assert_eq!(
            live_exe_path(PathBuf::from("/usr/local/bin/pairee (deleted)")),
            PathBuf::from("/usr/local/bin/pairee")
        );
        assert_eq!(
            live_exe_path(PathBuf::from("/usr/local/bin/pairee")),
            PathBuf::from("/usr/local/bin/pairee")
        );
    }
}
