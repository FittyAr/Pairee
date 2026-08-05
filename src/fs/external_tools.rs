use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// The URL of the 7z-extra package for Windows (v26.01)
const SEVENZIP_WIN_URL: &str =
    "https://github.com/ip7z/7zip/releases/download/26.01/7z2601-extra.7z";

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

    // 2. Save it to a temporary file. Use a UUID to avoid colliding with
    // any other process that might be downloading the same archive.
    let unique = uuid::Uuid::new_v4();
    let temp_archive = std::env::temp_dir().join(format!("pairee_7z_extra_{}.7z", unique));
    fs::write(&temp_archive, &response)?;

    // 3. Extract into a scratch directory, then copy out just the 7za.exe
    //    we need.
    //
    //    The previous implementation called
    //    `decompress_file_with_extract_fn` with a callback that returned
    //    `Ok(true)` for every non-target entry. That return value means
    //    "I have handled this entry, keep going", but the callback did
    //    not actually extract the entry — so the library never wrote the
    //    file. Worse, because the function never returned `Ok(false)`,
    //    processing continued through every entry in the (multi-megabyte)
    //    7-Zip distribution archive even though we only want one binary.
    //
    //    The fix: extract the whole archive into a scratch dir using the
    //    library's default extractor, then move the single 7za.exe we
    //    care about into `bin_path` and drop the rest. This is robust
    //    against future 7-Zip releases that change the layout.
    let scratch_dir = std::env::temp_dir().join(format!("pairee_7z_scratch_{}", unique));
    fs::create_dir_all(&scratch_dir)?;
    let extract_result = sevenz_rust::decompress_file(&temp_archive, &scratch_dir)
        .context("Failed to extract 7z-extra archive");

    // Whether or not the extraction succeeded, clean up the temp
    // archive as soon as possible.
    let _ = fs::remove_file(&temp_archive);

    extract_result?;

    // 4. Locate 7za.exe inside the scratch tree and move it into place.
    let extracted = scratch_dir.join("x64").join("7za.exe");
    if !extracted.exists() {
        // Try the older layout (7z extra used to ship `7za.exe` at the root).
        let alt = scratch_dir.join("7za.exe");
        if alt.exists() {
            fs::copy(&alt, &bin_path)
                .context("Failed to copy 7za.exe to bin directory")?;
        } else {
            let _ = fs::remove_dir_all(&scratch_dir);
            anyhow::bail!(
                "Downloaded 7z-extra archive does not contain 7za.exe (expected at x64/7za.exe)"
            );
        }
    } else {
        fs::copy(&extracted, &bin_path).context("Failed to copy 7za.exe to bin directory")?;
    }

    // 5. Clean up the scratch directory and the temp archive.
    let _ = fs::remove_dir_all(&scratch_dir);

    Ok(())
}
