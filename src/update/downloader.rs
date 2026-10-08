use anyhow::{Context as _, Result};
use tokio::sync::mpsc;

/// Only release assets published under this prefix are ever downloaded.
pub const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/FittyAr/Pairee/releases/download/";

/// Upper bound for a single downloaded release asset, so a hostile or
/// broken server cannot exhaust memory.
pub const MAX_ASSET_BYTES: u64 = 256 * 1024 * 1024;

/// Rejects any asset URL that is not a plain HTTPS download from the
/// official Pairee releases. Asset URLs come from the GitHub API or from the
/// on-disk release cache, so they are validated before every download.
pub fn validate_release_url(url: &str) -> Result<()> {
    let rest = url
        .strip_prefix(RELEASE_DOWNLOAD_PREFIX)
        .ok_or_else(|| anyhow::anyhow!("refusing to download from untrusted URL: {url}"))?;
    let suspicious = rest.is_empty()
        || rest.contains("..")
        || rest
            .chars()
            .any(|c| matches!(c, '?' | '#' | '@' | '\\' | '%') || c.is_whitespace());
    if suspicious {
        anyhow::bail!("refusing to download from malformed release URL: {url}");
    }
    Ok(())
}

/// Download a release asset from `url` into memory.
/// Sends progress updates (0.0 – 1.0) via `progress_tx` (may be None).
///
/// The bytes are returned instead of written to disk so the caller can verify
/// the checksum and extract/install from the very same buffer (no window in
/// which a file on disk could be swapped between verification and use).
pub async fn download_bytes(url: &str, progress_tx: Option<mpsc::Sender<f32>>) -> Result<Vec<u8>> {
    validate_release_url(url)?;

    let client = build_client()?;
    let mut response = client
        .get(url)
        .header(
            "User-Agent",
            format!("pairee/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .context("failed to start download")?;

    if !response.status().is_success() {
        anyhow::bail!("download returned status {}", response.status());
    }

    let total = response.content_length().unwrap_or(0);
    if total > MAX_ASSET_BYTES {
        anyhow::bail!("release asset is too large ({total} bytes)");
    }
    let mut data = Vec::with_capacity(total as usize);

    while let Some(chunk) = response.chunk().await.context("stream error")? {
        data.extend_from_slice(&chunk);
        if data.len() as u64 > MAX_ASSET_BYTES {
            anyhow::bail!("release asset exceeds {MAX_ASSET_BYTES} bytes");
        }
        if total > 0
            && let Some(tx) = &progress_tx
        {
            let _ = tx.try_send(data.len() as f32 / total as f32);
        }
    }

    if let Some(tx) = &progress_tx {
        let _ = tx.try_send(1.0);
    }

    Ok(data)
}

/// SHA-256 of an in-memory buffer as lowercase hex.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest as _;
    hex_encode(&sha2::Sha256::digest(data))
}

/// Compute the SHA-256 checksum string (hex, 64 chars) of a file.
pub fn compute_sha256(file_path: &std::path::Path) -> Result<String> {
    use sha2::Digest as _;
    let mut file = std::fs::File::open(file_path).context("failed to open file for hashing")?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher).context("read error during hashing")?;
    Ok(hex_encode(&hasher.finalize()))
}

/// Extracts the hex digest from the contents of a `.sha256` file
/// (`<hex>` or `<hex>  <filename>`). Fails unless it is exactly 64 hex chars.
pub fn parse_sha256_file(contents: &str) -> Result<String> {
    let digest = contents.split_whitespace().next().unwrap_or("");
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("malformed SHA-256 checksum file");
    }
    Ok(digest.to_ascii_lowercase())
}

/// Verify an in-memory buffer against a SHA-256 checksum string (hex, 64 chars).
pub fn verify_sha256_bytes(data: &[u8], expected_hex: &str) -> Result<()> {
    let actual_hex = sha256_hex(data);
    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        anyhow::bail!(
            "SHA-256 mismatch: expected {}, got {}",
            expected_hex,
            actual_hex
        )
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ─── Platform target selection ───────────────────────────────────────────────

/// Returns the expected asset filename for the current platform and target.
pub fn expected_asset_name(version: &str) -> String {
    let target = current_target();
    // Linux: pairee-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz
    // Windows: pairee-vX.Y.Z-x86_64-pc-windows-msvc.zip
    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("pairee-v{}-{}.{}", version, target, ext)
}

/// Returns the Inno Setup installer asset filename for Windows.
#[cfg(target_os = "windows")]
pub fn expected_installer_name(version: &str) -> String {
    if cfg!(target_arch = "aarch64") {
        format!("pairee-setup-{}-arm64.exe", version)
    } else {
        format!("pairee-setup-{}-x64.exe", version)
    }
}

/// Returns the current target triple for this binary.
fn current_target() -> &'static str {
    env!("PAIREE_TARGET")
}

fn build_client() -> Result<reqwest::Client> {
    use std::time::Duration;
    reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&million_a),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn compute_sha256_matches_in_memory_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(compute_sha256(&path).unwrap(), sha256_hex(b"abc"));
    }

    #[test]
    fn verify_sha256_bytes_detects_mismatch() {
        let good = sha256_hex(b"payload");
        assert!(verify_sha256_bytes(b"payload", &good).is_ok());
        assert!(verify_sha256_bytes(b"payload", &good.to_uppercase()).is_ok());
        assert!(verify_sha256_bytes(b"tampered", &good).is_err());
    }

    #[test]
    fn parse_sha256_file_accepts_common_formats() {
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(parse_sha256_file(hex).unwrap(), hex);
        assert_eq!(
            parse_sha256_file(&format!("{}  pairee.tar.gz\n", hex.to_uppercase())).unwrap(),
            hex
        );
        assert!(parse_sha256_file("").is_err());
        assert!(parse_sha256_file("not-a-digest pairee.tar.gz").is_err());
        assert!(parse_sha256_file(&hex[..63]).is_err());
    }

    #[test]
    fn release_url_must_be_official_https() {
        assert!(
            validate_release_url(
                "https://github.com/FittyAr/Pairee/releases/download/v0.9.0/pairee-v0.9.0-x86_64-unknown-linux-musl.tar.gz"
            )
            .is_ok()
        );
        for bad in [
            "http://github.com/FittyAr/Pairee/releases/download/v1/a.zip",
            "https://github.com/Evil/Pairee/releases/download/v1/a.zip",
            "https://github.com.evil.com/FittyAr/Pairee/releases/download/v1/a.zip",
            "https://github.com/FittyAr/Pairee/releases/download/../../Evil/x",
            "https://github.com/FittyAr/Pairee/releases/download/v1/a.zip?x=1",
            "https://github.com/FittyAr/Pairee/releases/download/v1/%2e%2e/a.zip",
            "https://github.com/FittyAr/Pairee/releases/download/",
            "file:///etc/passwd",
        ] {
            assert!(validate_release_url(bad).is_err(), "{bad}");
        }
    }
}
