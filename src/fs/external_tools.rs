use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// The URL of the 7z-extra package for Windows (v26.01)
const SEVENZIP_WIN_URL: &str =
    "https://github.com/ip7z/7zip/releases/download/26.01/7z2601-extra.7z";

/// Pinned SHA-256 of `SEVENZIP_WIN_URL` (GitHub release asset digest).
/// Must be updated together with the URL.
const SEVENZIP_WIN_SHA256: &str =
    "05cda5442075a7c6ce246ca1bbb9b1f1d6f1787a9559156f9b8b2dad29a86971";

/// Rejects a downloaded 7z-extra archive whose hash is not the pinned one.
fn verify_sevenzip_archive(data: &[u8]) -> Result<()> {
    crate::update::downloader::verify_sha256_bytes(data, SEVENZIP_WIN_SHA256)
        .context("Downloaded 7z-extra archive failed integrity check")
}

/// Gets the local path where `7za.exe` (or `7z`) should reside.
pub fn get_external_7z_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        let proj_dirs = ProjectDirs::from("com", "FittyAr", "Pairee")?;
        Some(proj_dirs.data_dir().join("bin").join("7za.exe"))
    } else {
        // On Linux/macOS, we rely on the system's `7z` or `7za` command
        Some(PathBuf::from("7z"))
    }
}

/// Downloads and extracts the 7-Zip standalone executable on Windows.
/// On Linux/macOS, this is a no-op as we assume system packages are used.
pub async fn ensure_external_tools() -> Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(()); // Handled by system packages on UNIX
    }

    let bin_path = get_external_7z_path().context("Could not determine bin path")?;

    // If it already exists and size > 1MB, it's valid
    if bin_path.exists() {
        if let Ok(metadata) = fs::metadata(&bin_path)
            && metadata.len() > 1024 * 1024
        {
            return Ok(());
        }
        // If it's too small (like a 404 page), remove it and re-download
        let _ = fs::remove_file(&bin_path);
    }

    // Ensure bin folder exists
    if let Some(parent) = bin_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 1. Download the 7z archive into memory and verify the pinned hash
    //    before anything is written to disk or extracted.
    let response = reqwest::get(SEVENZIP_WIN_URL)
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    verify_sevenzip_archive(&response)?;

    // 2. Extract the verified bytes into a private, randomly named scratch
    //    directory (removed on drop), then copy out just the 7za.exe we need.
    //    Extracting the whole archive with the library's default extractor
    //    is robust against future 7-Zip releases that change the layout.
    let scratch = tempfile::Builder::new()
        .prefix("pairee_7z_scratch_")
        .tempdir()?;
    let scratch_dir = scratch.path();
    sevenz_rust2::decompress(std::io::Cursor::new(&response[..]), scratch_dir)
        .context("Failed to extract 7z-extra archive")?;

    // 3. Locate 7za.exe inside the scratch tree and move it into place.
    let extracted = scratch_dir.join("x64").join("7za.exe");
    if !extracted.exists() {
        // Try the older layout (7z extra used to ship `7za.exe` at the root).
        let alt = scratch_dir.join("7za.exe");
        if alt.exists() {
            fs::copy(&alt, &bin_path).context("Failed to copy 7za.exe to bin directory")?;
        } else {
            anyhow::bail!(
                "Downloaded 7z-extra archive does not contain 7za.exe (expected at x64/7za.exe)"
            );
        }
    } else {
        fs::copy(&extracted, &bin_path).context("Failed to copy 7za.exe to bin directory")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_hash_is_well_formed() {
        assert_eq!(SEVENZIP_WIN_SHA256.len(), 64);
        assert!(SEVENZIP_WIN_SHA256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn tampered_archive_is_rejected() {
        assert!(verify_sevenzip_archive(b"not the 7z-extra archive").is_err());
    }
}
