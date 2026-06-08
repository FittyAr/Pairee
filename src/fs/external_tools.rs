use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;
use directories::ProjectDirs;

/// The URL of the 7z-extra package for Windows (v26.01)
const SEVENZIP_WIN_URL: &str = "https://github.com/ip7z/7zip/releases/download/26.01/7z2601-extra.7z";

/// Gets the local path where `7za.exe` (or `7z`) should reside.
pub fn get_external_7z_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        let proj_dirs = ProjectDirs::from("com", "FittyAr", "NCRust")?;
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
        if let Ok(metadata) = fs::metadata(&bin_path) {
            if metadata.len() > 1024 * 1024 {
                return Ok(());
            }
        }
        // If it's too small (like a 404 page), remove it and re-download
        let _ = fs::remove_file(&bin_path);
    }

    // Ensure bin folder exists
    if let Some(parent) = bin_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 1. Download the 7z archive
    let response = reqwest::get(SEVENZIP_WIN_URL).await?.bytes().await?;
    
    // 2. Save it to a temporary file
    let temp_archive = std::env::temp_dir().join("ncrust_7z_extra.7z");
    fs::write(&temp_archive, &response)?;

    // 3. Extract 7za.exe using our internal sevenz-rust crate
    sevenz_rust::decompress_file_with_extract_fn(&temp_archive, bin_path.parent().unwrap(), |entry, reader, _dest| {
        // Only extract the specific 64-bit 7za.exe
        if entry.name().eq_ignore_ascii_case("x64/7za.exe") {
            // Write it directly to the bin_path destination
            let mut file = fs::File::create(&bin_path)?;
            std::io::copy(reader, &mut file)?;
            return Ok(true);
        }
        
        Ok(true) // skip others but continue extraction process
    }).context("Failed to extract 7za.exe from downloaded archive")?;

    // Cleanup temp file
    let _ = fs::remove_file(temp_archive);

    Ok(())
}
