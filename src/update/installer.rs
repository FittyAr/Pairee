use anyhow::{Context as _, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::detect::InstallMethod;
use super::downloader;
use crate::update::UpdateInfo;

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

    // --- Create a temp directory for downloads ---
    let temp_dir = create_temp_dir()?;

    // --- Pick the right asset ---
    #[cfg(target_os = "windows")]
    let (asset_name, use_installer) = {
        if matches!(method, InstallMethod::InnoSetup) {
            (
                downloader::expected_installer_name(&info.version),
                true,
            )
        } else {
            (downloader::expected_asset_name(&info.version), false)
        }
    };

    #[cfg(not(target_os = "windows"))]
    let (asset_name, _use_installer) = (downloader::expected_asset_name(&info.version), false);

    let asset = info
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Release does not contain expected asset '{}'. Available: {}",
                asset_name,
                info.assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // --- Download ---
    let downloaded = downloader::download_asset(
        &asset.browser_download_url,
        &temp_dir,
        &asset.name,
        Some(progress_tx),
    )
    .await
    .context("download failed")?;

    // --- Optionally verify SHA-256 ---
    let sha_asset_name = format!("{}.sha256", asset_name);
    if let Some(sha_asset) = info.assets.iter().find(|a| a.name == sha_asset_name) {
        let sha_file = downloader::download_asset(
            &sha_asset.browser_download_url,
            &temp_dir,
            &sha_asset.name,
            None,
        )
        .await
        .context("failed to download SHA256 file")?;
        let expected = std::fs::read_to_string(&sha_file)
            .context("failed to read SHA256 file")?;
        let expected = expected.split_whitespace().next().unwrap_or("").to_string();
        if !expected.is_empty() {
            downloader::verify_sha256(&downloaded, &expected)
                .context("SHA-256 verification failed")?;
        }
    }

    // --- Install ---
    #[cfg(target_os = "windows")]
    {
        if use_installer {
            install_windows_inno(&downloaded).await
        } else {
            install_windows_zip(&downloaded).await
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        install_linux_tarball(&downloaded).await
    }
}

// ─── Linux: replace binary from tar.gz ───────────────────────────────────────

#[cfg(not(target_os = "windows"))]
async fn install_linux_tarball(archive: &Path) -> Result<InstallResult> {
    use std::fs;

    let exe = std::env::current_exe().context("cannot determine current exe path")?;
    let exe_dir = exe.parent().context("exe has no parent directory")?;

    // Extract into a sibling temp dir
    let extract_dir = exe_dir.join(".pairee_update_tmp");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir).context("cannot create extract directory")?;

    // Use std tar + flate2 (already in Cargo.toml)
    extract_tar_gz(archive, &extract_dir).context("failed to extract tar.gz")?;

    // Find the pairee binary inside the extracted tree
    let new_bin = find_binary_in_dir(&extract_dir, "pairee")
        .ok_or_else(|| anyhow::anyhow!("pairee binary not found in extracted archive"))?;

    // Atomic replace: copy over the existing exe (safe on Linux even if running)
    let backup = exe.with_extension("bak");
    let _ = fs::rename(&exe, &backup); // may fail if read-only, that's OK — copy fallback
    fs::copy(&new_bin, &exe).context("failed to replace pairee binary")?;
    let _ = set_executable(&exe);

    // Clean up
    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(backup);

    Ok(InstallResult::RestartRequired)
}

#[cfg(not(target_os = "windows"))]
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive).context("failed to open archive")?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    tar.unpack(dest).context("failed to unpack archive")?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn find_binary_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in walkdir_simple(dir) {
        if entry.file_name().to_string_lossy() == name {
            if entry.metadata().map(|m| !m.is_dir()).unwrap_or(false) {
                return Some(entry.path().to_path_buf());
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn walkdir_simple(dir: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
    // Simple recursive walk without external crate
    let mut stack = vec![dir.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for entry in rd.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    stack.push(entry.path());
                }
                entries.push(entry);
            }
        }
    }
    entries.into_iter()
}

#[cfg(not(target_os = "windows"))]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

// ─── Windows: replace binary from zip ────────────────────────────────────────

#[cfg(target_os = "windows")]
async fn install_windows_zip(archive: &Path) -> Result<InstallResult> {
    let exe = std::env::current_exe().context("cannot determine current exe path")?;
    let install_dir = exe.parent().context("exe has no parent directory")?;

    let extract_dir = install_dir.join(".pairee_update_tmp");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).context("cannot create extract directory")?;

    // Extract zip
    let file = std::fs::File::open(archive).context("failed to open zip")?;
    let mut zip = zip::ZipArchive::new(file).context("failed to read zip archive")?;
    zip.extract(&extract_dir).context("failed to extract zip")?;

    // Find new pairee.exe
    let new_exe = find_binary_in_dir_windows(&extract_dir)
        .ok_or_else(|| anyhow::anyhow!("pairee.exe not found in archive"))?;

    // Write a small .bat helper that will replace the exe after Pairee exits
    let helper_bat = install_dir.join("pairee_update.bat");
    let bat_content = format!(
        "@echo off\r\ntimeout /t 2 /nobreak >nul\r\ncopy /y \"{}\" \"{}\"\r\ndel \"%~f0\"\r\nstart \"\" \"{}\"\r\n",
        new_exe.display(),
        exe.display(),
        exe.display()
    );
    std::fs::write(&helper_bat, bat_content).context("failed to write update helper")?;

    // Launch the bat detached
    std::process::Command::new("cmd")
        .args(["/c", "start", "", helper_bat.to_str().unwrap_or("")])
        .spawn()
        .context("failed to launch update helper")?;

    Ok(InstallResult::WindowsInstallerLaunched)
}

#[cfg(target_os = "windows")]
fn find_binary_in_dir_windows(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name == "pairee.exe" {
                    return Some(entry.path());
                }
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    stack.push(entry.path());
                }
            }
        }
    }
    None
}

/// Windows Inno Setup silent install — downloads and runs the installer.
/// The installer replaces the binary and handles the rest.
#[cfg(target_os = "windows")]
async fn install_windows_inno(installer: &Path) -> Result<InstallResult> {
    std::process::Command::new(installer)
        .args(["/verysilent", "/update=true", "/MERGETASKS=!desktopicon"])
        .spawn()
        .context("failed to launch Inno Setup installer")?;
    Ok(InstallResult::WindowsInstallerLaunched)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn create_temp_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir().join("pairee_update");
    std::fs::create_dir_all(&base).context("failed to create temp dir")?;
    Ok(base)
}
