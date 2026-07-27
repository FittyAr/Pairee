use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// The URL of the 7z-extra package for Windows (v26.01)
const SEVENZIP_WIN_URL: &str =
    "https://github.com/ip7z/7zip/releases/download/26.01/7z2601-extra.7z";

/// Hard cap on the 7z archive we will download (~50 MiB is plenty for
/// the standalone 7za.exe; the whole extra bundle is ~30 MiB).
const MAX_SEVENZIP_BYTES: u64 = 50 * 1024 * 1024;

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

    // 1. Download the 7z archive with a hard size cap and a request
    //    timeout. The old `reqwest::get` had no timeout and no size
    //    limit, which meant a malicious or hung server could fill
    //    the user's disk or hang the app indefinitely.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("failed to build 7z download client")?;
    let mut response = client
        .get(SEVENZIP_WIN_URL)
        .header(
            "User-Agent",
            format!("pairee/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .context("failed to start 7z download")?;

    if !response.status().is_success() {
        anyhow::bail!("7z download returned status {}", response.status());
    }
    if let Some(len) = response.content_length() {
        if len > MAX_SEVENZIP_BYTES {
            anyhow::bail!(
                "7z archive is too large: {} bytes advertised (max {})",
                len,
                MAX_SEVENZIP_BYTES
            );
        }
    }

    // 2. Save it to a *per-process* temp file. The previous code
    //    used a fixed name (`pairee_7z_extra.7z`) which races with
    //    any other Pairee instance and with any leftover file from
    //    a previous crash. We now use the PID in the filename and
    //    register a cleanup closure so the temp file is removed on
    //    every exit path (success, error, panic).
    let temp_archive =
        std::env::temp_dir().join(format!("pairee_7z_extra_{}.7z", std::process::id()));
    struct TempGuard(PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _guard = TempGuard(temp_archive.clone());

    {
        use std::io::Write as _;
        let mut file = fs::File::create(&temp_archive).context("failed to create 7z temp file")?;
        let mut written: u64 = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .context("error while streaming 7z archive")?
        {
            written = written
                .checked_add(chunk.len() as u64)
                .context("7z byte counter overflow")?;
            if written > MAX_SEVENZIP_BYTES {
                anyhow::bail!(
                    "7z archive exceeded the {} byte cap mid-stream",
                    MAX_SEVENZIP_BYTES
                );
            }
            file.write_all(&chunk).context("failed to write 7z chunk")?;
        }
        file.flush().context("failed to flush 7z temp file")?;
    }

    // 3. Extract 7za.exe using our internal sevenz-rust crate
    sevenz_rust::decompress_file_with_extract_fn(
        &temp_archive,
        bin_path.parent().unwrap(),
        |entry, reader, _dest| {
            // Only extract the specific 64-bit 7za.exe
            if entry.name().eq_ignore_ascii_case("x64/7za.exe") {
                // Write it directly to the bin_path destination
                let mut file = fs::File::create(&bin_path)?;
                std::io::copy(reader, &mut file)?;
                return Ok(true);
            }

            Ok(true) // skip others but continue extraction process
        },
    )
    .context("Failed to extract 7za.exe from downloaded archive")?;

    // _guard drops here and removes the temp file.

    Ok(())
}
