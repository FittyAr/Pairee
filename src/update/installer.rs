use anyhow::{Context as _, Result};
#[cfg(not(target_os = "windows"))]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::detect::InstallMethod;
use super::downloader;
use crate::update::UpdateInfo;
use crate::update::checker::ReleaseAsset;

/// Result of a completed update.
#[derive(Debug)]
pub enum InstallResult {
    /// The update was applied in-place. The user should restart Pairee.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    RestartRequired,
    /// On Windows, the installer has been invoked. Pairee will close.
    #[cfg(target_os = "windows")]
    WindowsInstallerLaunched,
    /// The managed package-manager command was shown to the user (no action taken here).
    ManagedCommandShown,
}

/// Download and apply an update.
///
/// Progress (0.0–1.0) is sent over `progress_tx` during the download phase.
/// After download, the appropriate installer is invoked.
pub async fn perform_update(
    info: &UpdateInfo,
    method: &InstallMethod,
    progress_tx: mpsc::Sender<f32>,
) -> Result<InstallResult> {
    if method.is_managed() {
        // Nothing to do on this side — the UI has already shown the command.
        return Ok(InstallResult::ManagedCommandShown);
    }

    // --- Pick the right asset ---
    #[cfg(target_os = "windows")]
    let (asset_name, use_installer) = {
        if matches!(method, InstallMethod::InnoSetup) {
            (downloader::expected_installer_name(&info.version), true)
        } else {
            (downloader::expected_asset_name(&info.version), false)
        }
    };

    #[cfg(not(target_os = "windows"))]
    let (asset_name, _use_installer) = (downloader::expected_asset_name(&info.version), false);

    let asset = find_asset(info, &asset_name, "asset")?;
    // Checksum and signature are mandatory: fail closed without them.
    let sha_asset = find_asset(info, &format!("{asset_name}.sha256"), "checksum")?;
    let sig_asset = find_asset(info, &format!("{asset_name}.minisig"), "signature")?;

    // --- Download into memory and verify ---
    // Install/extract strictly from the verified buffer, so nothing on disk
    // can be swapped between verification and use.
    let data = downloader::download_bytes(&asset.browser_download_url, Some(progress_tx))
        .await
        .context("download failed")?;
    let sha_text = download_text(sha_asset).await?;
    let expected = downloader::parse_sha256_file(&sha_text)?;
    downloader::verify_sha256_bytes(&data, &expected).context("SHA-256 verification failed")?;
    let sig_text = download_text(sig_asset).await?;
    super::signature::verify_release_asset(&data, &sig_text, &asset.name)
        .context("signature verification failed")?;

    // --- Install ---
    #[cfg(target_os = "windows")]
    {
        if use_installer {
            install_windows_inno(&data, &asset.name)
        } else {
            install_windows_zip(&data)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        install_linux_tarball(&data)
    }
}

/// Find a release asset by exact name; refusing to continue without it.
fn find_asset<'a>(info: &'a UpdateInfo, name: &str, what: &str) -> Result<&'a ReleaseAsset> {
    info.assets.iter().find(|a| a.name == name).ok_or_else(|| {
        anyhow::anyhow!(
            "Release does not contain {what} '{name}'; refusing to install an unverified update. Available: {}",
            info.assets
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Download a small UTF-8 side file (checksum / signature).
async fn download_text(asset: &ReleaseAsset) -> Result<String> {
    let bytes = downloader::download_bytes(&asset.browser_download_url, None)
        .await
        .with_context(|| format!("failed to download {}", asset.name))?;
    String::from_utf8(bytes).with_context(|| format!("{} is not valid UTF-8", asset.name))
}

// ─── Linux: replace binary from tar.gz ───────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn install_linux_tarball(archive: &[u8]) -> Result<InstallResult> {
    use std::io::Write as _;

    let exe = std::env::current_exe().context("cannot determine current exe path")?;
    let exe_dir = exe.parent().context("exe has no parent directory")?;

    let new_bin = extract_file_from_tar_gz(archive, "pairee")?;

    // Write to a randomly named sibling (created 0600 by tempfile), mark it
    // executable and atomically rename it over the running binary.
    let mut staged = tempfile::Builder::new()
        .prefix(".pairee_update_")
        .tempfile_in(exe_dir)
        .context("cannot create staging file next to pairee binary")?;
    staged
        .write_all(&new_bin)
        .context("failed to write new pairee binary")?;
    staged.as_file().sync_all().ok();
    set_executable(staged.path())?;
    staged
        .persist(&exe)
        .map_err(|e| e.error)
        .context("failed to replace pairee binary")?;

    Ok(InstallResult::RestartRequired)
}

/// Returns the contents of the first regular file named `name` in a tar.gz
/// buffer. Only that single file is read; nothing else is written to disk.
#[cfg(any(not(target_os = "windows"), test))]
fn extract_file_from_tar_gz(archive: &[u8], name: &str) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(archive));
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries().context("failed to read archive")? {
        let mut entry = entry.context("corrupt archive entry")?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = entry.path().context("invalid archive path")?;
        if path.file_name().is_some_and(|n| n == name) {
            let mut out = Vec::new();
            entry
                .by_ref()
                .take(downloader::MAX_ASSET_BYTES)
                .read_to_end(&mut out)
                .context("failed to read binary from archive")?;
            return Ok(out);
        }
    }
    anyhow::bail!("{name} binary not found in extracted archive")
}

#[cfg(not(target_os = "windows"))]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

// ─── Windows: replace binary from zip ────────────────────────────────────────

/// Private, randomly named work dir for files that must outlive Pairee
/// (the replacement exe and its helper script, or the installer). It is
/// deliberately kept on disk because the helper runs after Pairee exits.
#[cfg(target_os = "windows")]
fn create_work_dir() -> Result<PathBuf> {
    let dir = tempfile::Builder::new()
        .prefix("pairee_update_")
        .tempdir()
        .context("failed to create temp dir")?;
    Ok(dir.keep())
}

#[cfg(target_os = "windows")]
fn install_windows_zip(archive: &[u8]) -> Result<InstallResult> {
    let exe = std::env::current_exe().context("cannot determine current exe path")?;
    let work_dir = create_work_dir()?;

    let new_exe_bytes = extract_exe_from_zip(archive)?;
    let new_exe = work_dir.join("pairee.exe");
    write_new_file(&new_exe, &new_exe_bytes)?;

    // Write a small .bat helper that will replace the exe after Pairee exits.
    //
    // Both `new_exe` and `exe` are paths derived from the downloaded
    // archive (the archive controls the directory layout of `new_exe`).
    // Naively interpolating them into a `.bat` body is a code-injection
    // vector: cmd.exe treats `&`, `|`, `<`, `>`, `^`, `!`, `%` and
    // unbalanced `"` as metacharacters inside batch files. A directory
    // named e.g. `& calc` in the archive would be rendered as
    // `copy /y "C:\extract\& calc\pairee.exe" "..."` and the `&` would
    // chain an extra `calc` invocation.
    //
    // To eliminate this, the helper does not mention either path in the
    // script body. Instead the paths are passed as `%~1` and `%~2` and
    // expanded by cmd at runtime, after cmd's batch parser has finished
    // its pass over the literal body. This keeps the literal body
    // entirely metacharacter-free.
    let helper_bat = work_dir.join("pairee_update.bat");
    let bat_content = "@echo off\r\n\
                       setlocal DisableDelayedExpansion\r\n\
                       timeout /t 2 /nobreak >nul\r\n\
                       copy /y \"%~1\" \"%~2\"\r\n\
                       del \"%~f0\"\r\n\
                       start \"\" \"%~2\"\r\n";
    write_new_file(&helper_bat, bat_content.as_bytes()).context("failed to write update helper")?;

    // Launch the bat detached. The two paths are appended to the bat's
    // argv so they reach the script as `%~1` and `%~2` (see the comment
    // above for why we don't interpolate them into the script body).
    let helper_bat_str = helper_bat.to_string_lossy().into_owned();
    let new_exe_str = new_exe.to_string_lossy().into_owned();
    let exe_str = exe.to_string_lossy().into_owned();
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &helper_bat_str, &new_exe_str, &exe_str])
        .spawn()
        .context("failed to launch update helper")?;

    Ok(InstallResult::WindowsInstallerLaunched)
}

/// Reads `pairee.exe` (any directory level) out of a zip buffer.
#[cfg(any(target_os = "windows", test))]
fn extract_exe_from_zip(archive: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
        .context("failed to read zip archive")?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).context("corrupt zip entry")?;
        let is_exe = file
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .is_some_and(|n| n == "pairee.exe");
        if is_exe && file.is_file() {
            let mut out = Vec::new();
            (&mut file)
                .take(downloader::MAX_ASSET_BYTES)
                .read_to_end(&mut out)
                .context("failed to read pairee.exe from zip")?;
            return Ok(out);
        }
    }
    anyhow::bail!("pairee.exe not found in archive")
}

/// Writes `data` to a path that must not exist yet (no clobbering, no
/// following a pre-planted file).
#[cfg(target_os = "windows")]
fn write_new_file(path: &Path, data: &[u8]) -> Result<()> {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}

/// Windows Inno Setup silent install — runs the verified installer.
/// The installer replaces the binary and handles the rest.
#[cfg(target_os = "windows")]
fn install_windows_inno(installer: &[u8], asset_name: &str) -> Result<InstallResult> {
    let work_dir = create_work_dir()?;
    let installer_path = work_dir.join(asset_name);
    write_new_file(&installer_path, installer)?;
    std::process::Command::new(&installer_path)
        .args(["/verysilent", "/update=true", "/MERGETASKS=!desktopicon"])
        .spawn()
        .context("failed to launch Inno Setup installer")?;
    Ok(InstallResult::WindowsInstallerLaunched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn tar_gz_extracts_only_named_binary() {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, body) in [("pkg/README", &b"readme"[..]), ("pkg/pairee", &b"BIN"[..])] {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, body).unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar_bytes).unwrap();
        let archive = gz.finish().unwrap();

        assert_eq!(
            extract_file_from_tar_gz(&archive, "pairee").unwrap(),
            b"BIN"
        );
        assert!(extract_file_from_tar_gz(&archive, "missing").is_err());
    }

    #[test]
    fn zip_extracts_pairee_exe() {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("dist/Pairee.exe", opts).unwrap();
        zip.write_all(b"EXE").unwrap();
        let archive = zip.finish().unwrap().into_inner();

        assert_eq!(extract_exe_from_zip(&archive).unwrap(), b"EXE");
    }
}
