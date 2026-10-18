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

/// Most bytes of the 7z-extra download read into memory (it is ~1 MiB).
const SEVENZIP_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Time allowed for the whole download.
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Held while the tool is bootstrapped: two callers (every archive open may
/// ask) must not download and write the same file at once.
static BOOTSTRAP: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Where the downloaded `7za.exe` lives (Windows; elsewhere the system's
/// 7-Zip is used, see [`find_7z`]).
pub fn get_external_7z_path() -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let proj_dirs = ProjectDirs::from("com", "FittyAr", "Pairee")?;
    Some(proj_dirs.data_dir().join("bin").join("7za.exe"))
}

/// A 7-Zip command-line program.
#[derive(Debug, Clone)]
pub struct SevenZipTool {
    pub path: PathBuf,
    /// It reads every format 7-Zip knows (`7z`, `7zz`). The standalone
    /// `7za` Pairee downloads reads only 7z, zip, tar, gzip, bzip2, xz and
    /// cab archives: no RAR, no ISO images.
    pub full: bool,
}

/// The 7-Zip to run for archives without a native reader: an installed
/// full 7-Zip (on `PATH`, or in its usual Windows folder) when there is
/// one, else the reduced standalone `7za` (downloaded on Windows, from
/// `PATH` elsewhere).
pub fn find_7z() -> Option<SevenZipTool> {
    let (full_names, reduced) = if cfg!(windows) {
        (&["7z.exe", "7zz.exe"][..], get_external_7z_path())
    } else {
        (&["7zz", "7z"][..], in_path(&["7za"]))
    };
    in_path(full_names)
        .or_else(installed_7z)
        .map(|path| SevenZipTool { path, full: true })
        .or_else(|| {
            reduced
                .filter(|p| p.is_file())
                .map(|path| SevenZipTool { path, full: false })
        })
}

/// The first of `names` found in a `PATH` folder.
fn in_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// 7-Zip installed in its default folder (Windows; not added to `PATH`
/// by its installer).
fn installed_7z() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|dir| PathBuf::from(dir).join("7-Zip").join("7z.exe"))
        .find(|candidate| candidate.is_file())
}

/// Downloads and extracts the 7-Zip standalone executable on Windows.
/// On Linux/macOS, this is a no-op as we assume system packages are used.
pub async fn ensure_external_tools() -> Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(()); // Handled by system packages on UNIX
    }

    let bin_path = get_external_7z_path().context("Could not determine bin path")?;
    let _only_one = BOOTSTRAP.lock().await;

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
    let client = crate::update::http::client(DOWNLOAD_TIMEOUT)?;
    let response = client
        .get(SEVENZIP_WIN_URL)
        .send()
        .await?
        .error_for_status()?;
    let response =
        crate::update::http::read_limited(response, SEVENZIP_MAX_BYTES, |_, _| {}).await?;
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

    // 3. Locate 7za.exe inside the scratch tree (the older layout had it at
    //    the root) and put it in place under a temporary name first: a
    //    copy cut short by a crash or a full disk must never sit at the
    //    final name, where the size check above would take it for the tool.
    let extracted = [
        scratch_dir.join("x64").join("7za.exe"),
        scratch_dir.join("7za.exe"),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
    .context("Downloaded 7z-extra archive does not contain 7za.exe (expected at x64/7za.exe)")?;
    install_atomically(&extracted, &bin_path)
}

/// Copies `from` to `to` through a temporary file next to it, renamed over
/// `to` once complete.
fn install_atomically(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    let mut part = to.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let result = fs::copy(from, &part)
        .context("Failed to copy 7za.exe to bin directory")
        .and_then(|_| fs::rename(&part, to).context("Failed to put 7za.exe in place"));
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
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
    fn installing_replaces_through_a_temporary_name() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("src.exe"), dir.path().join("7za.exe"));
        std::fs::write(&from, b"new tool").unwrap();
        std::fs::write(&to, b"old").unwrap();
        install_atomically(&from, &to).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"new tool");
        assert!(!dir.path().join("7za.exe.part").exists());
        // A missing source leaves the target and no temporary file.
        assert!(install_atomically(&dir.path().join("none"), &to).is_err());
        assert_eq!(std::fs::read(&to).unwrap(), b"new tool");
        assert!(!dir.path().join("7za.exe.part").exists());
    }

    #[test]
    fn tampered_archive_is_rejected() {
        assert!(verify_sevenzip_archive(b"not the 7z-extra archive").is_err());
    }
}
