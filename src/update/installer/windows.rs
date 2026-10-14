//! Windows: the zip build replaces `pairee.exe` in place; an Inno Setup
//! install runs the new installer.
//!
//! Windows cannot overwrite or delete a running executable, but it can
//! rename it. The running exe is moved aside, the new one renamed into its
//! place, and the old one restored if that fails. The moved-aside file is
//! removed by [`super::remove_stale_files`] once no process runs it.

use anyhow::{Context as _, Result, anyhow};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::{InstallResult, cleanup, extract};

/// Runs the verified installer `asset_name` (`is_installer`), or replaces
/// the running exe with the one in the verified zip.
pub(super) fn install(data: &[u8], asset_name: &str, is_installer: bool) -> Result<InstallResult> {
    if is_installer {
        install_inno(data, asset_name)
    } else {
        let new_exe = extract::exe_from_zip(data)?;
        let exe = std::env::current_exe().context("cannot determine current exe path")?;
        replace_exe(&exe, &new_exe)?;
        Ok(InstallResult::RestartRequired)
    }
}

/// `std::fs::rename`, retried for a moment while another process (an
/// antivirus scanning the file, a process still mapping a just-started
/// image) holds it without delete sharing (`ERROR_SHARING_VIOLATION`).
fn rename_when_free(from: &Path, to: &Path) -> std::io::Result<()> {
    const SHARING_VIOLATION: i32 = 32;
    let mut delay = std::time::Duration::from_millis(25);
    for _ in 0..8 {
        match std::fs::rename(from, to) {
            Err(e) if e.raw_os_error() == Some(SHARING_VIOLATION) => {
                std::thread::sleep(delay);
                delay *= 2;
            }
            result => return result,
        }
    }
    std::fs::rename(from, to)
}

/// Replaces `exe` (possibly running) with `new_bytes`, restoring it when the
/// replacement cannot be put in place.
fn replace_exe(exe: &Path, new_bytes: &[u8]) -> Result<()> {
    let dir = exe.parent().context("exe has no parent directory")?;

    // Staged on the same volume, so the final step is a rename.
    let mut staged = tempfile::Builder::new()
        .prefix(cleanup::STAGING_PREFIX)
        .suffix(".exe")
        .tempfile_in(dir)
        .context("cannot create staging file next to pairee.exe")?;
    staged
        .write_all(new_bytes)
        .context("failed to write new pairee.exe")?;
    staged
        .as_file()
        .sync_all()
        .context("failed to flush new pairee.exe")?;

    let old = moved_aside_path(dir);
    rename_when_free(exe, &old).with_context(|| format!("cannot move {} aside", exe.display()))?;

    if let Err(e) = staged.persist(exe) {
        let failure = anyhow!(e.error);
        return Err(match std::fs::rename(&old, exe) {
            Ok(()) => failure.context("failed to install the new pairee.exe; the previous version was restored"),
            Err(restore) => failure.context(format!(
                "failed to install the new pairee.exe, and restoring the previous one from {} failed: {restore}",
                old.display()
            )),
        });
    }
    Ok(())
}

/// A name for the replaced exe that no other update uses.
fn moved_aside_path(dir: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    dir.join(format!(
        "{}old_{}_{nanos}.exe",
        cleanup::STAGING_PREFIX,
        std::process::id()
    ))
}

/// Windows Inno Setup silent install — runs the verified installer, which
/// replaces the binary and handles the rest. Its private work folder must
/// outlive Pairee; a later start removes it.
fn install_inno(installer: &[u8], asset_name: &str) -> Result<InstallResult> {
    let work_dir = tempfile::Builder::new()
        .prefix(cleanup::WORK_DIR_PREFIX)
        .tempdir()
        .context("failed to create temp dir")?
        .keep();
    let installer_path = work_dir.join(asset_name);
    write_new_file(&installer_path, installer)?;
    std::process::Command::new(&installer_path)
        .args(["/verysilent", "/update=true", "/MERGETASKS=!desktopicon"])
        .spawn()
        .context("failed to launch Inno Setup installer")?;
    Ok(InstallResult::WindowsInstallerLaunched)
}

/// Writes `data` to a path that must not exist yet (no clobbering, no
/// following a pre-planted file).
fn write_new_file(path: &Path, data: &[u8]) -> Result<()> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_exe_swaps_the_file_and_keeps_the_old_one_aside() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("pairee.exe");
        std::fs::write(&exe, b"OLD").unwrap();

        replace_exe(&exe, b"NEW").unwrap();

        assert_eq!(std::fs::read(&exe).unwrap(), b"NEW");
        let aside: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p != &exe)
            .collect();
        assert_eq!(aside.len(), 1, "{aside:?}");
        assert_eq!(std::fs::read(&aside[0]).unwrap(), b"OLD");
    }

    #[test]
    fn replace_exe_reports_a_missing_exe_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("pairee.exe");
        assert!(replace_exe(&exe, b"NEW").is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// Child process of `replace_exe_replaces_a_running_exe`.
    #[test]
    #[ignore = "helper process, started by replace_exe_replaces_a_running_exe"]
    fn running_exe_helper() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    #[test]
    fn replace_exe_replaces_a_running_exe() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("pairee.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        let mut child = std::process::Command::new(&exe)
            .args([
                "--ignored",
                "--exact",
                "update::installer::windows::tests::running_exe_helper",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        // The image is mapped once CreateProcess returns: a plain overwrite
        // (what the old .bat helper did) fails.
        let overwrite_failed = std::fs::write(&exe, b"NEW").is_err();
        let replaced = replace_exe(&exe, b"NEW");
        let _ = child.kill();
        let _ = child.wait();
        assert!(overwrite_failed);
        replaced.unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"NEW");
    }
}
