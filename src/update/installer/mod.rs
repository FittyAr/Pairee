//! Applies a downloaded update. The asset is verified (SHA-256 and minisign)
//! in memory here; [`platform`] then installs it from that same buffer.

mod cleanup;
mod extract;
#[cfg(not(target_os = "windows"))]
mod unix;
#[cfg(not(target_os = "windows"))]
use unix as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

use anyhow::{Context as _, Result};
use tokio::sync::mpsc;

use super::detect::InstallMethod;
use super::downloader;
use crate::update::UpdateInfo;
use crate::update::checker::ReleaseAsset;

pub use cleanup::remove_stale_files;

/// Result of a completed update.
#[derive(Debug)]
pub enum InstallResult {
    /// The update was applied in-place. The user should restart Pairee.
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
    tokio::task::spawn_blocking(remove_stale_files);

    let package = platform::package_for(&info.version, method);
    let asset = find_asset(info, &package.asset_name, "asset")?;
    // Checksum and signature are mandatory: fail closed without them.
    let sha_asset = find_asset(info, &format!("{}.sha256", package.asset_name), "checksum")?;
    let sig_asset = find_asset(
        info,
        &format!("{}.minisig", package.asset_name),
        "signature",
    )?;

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

    platform::install(&data, &package)
}

/// The release asset that updates this installation.
struct Package {
    asset_name: String,
    /// The asset is the Inno Setup installer rather than an archive.
    #[cfg(target_os = "windows")]
    is_installer: bool,
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
