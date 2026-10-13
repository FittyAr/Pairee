//! Linux and macOS: the binary in the release tarball replaces the running
//! one with an atomic rename (the running process keeps its old inode).

use anyhow::{Context as _, Result};
use std::io::Write as _;
use std::path::Path;

use super::{InstallResult, Package, cleanup, extract};
use crate::update::detect::InstallMethod;
use crate::update::downloader;

/// The tarball of this target.
pub(super) fn package_for(version: &str, _method: &InstallMethod) -> Package {
    Package {
        asset_name: downloader::expected_asset_name(version),
    }
}

pub(super) fn install(archive: &[u8], _package: &Package) -> Result<InstallResult> {
    let exe = std::env::current_exe().context("cannot determine current exe path")?;
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

fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}
