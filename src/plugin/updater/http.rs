//! Bounded downloads from the plugin registry. Every response is read in
//! chunks and refused as soon as it grows past its limit, so a hostile or
//! broken registry cannot make Pairee buffer an unbounded body.

use anyhow::{Context as _, Result};
use std::time::Duration;

/// Largest `index.toml` / `blocklist.toml` accepted.
pub const MAX_INDEX_BYTES: usize = 4 * 1024 * 1024;
/// Largest plugin `manifest.toml` (or `.minisig`) accepted.
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
/// Largest single plugin file accepted.
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
/// Largest plugin (all files together) accepted.
pub const MAX_PLUGIN_BYTES: usize = 64 * 1024 * 1024;
/// Most files one plugin may list in `[files]`.
pub const MAX_PLUGIN_FILES: usize = 1000;

/// HTTP client for registry requests.
pub fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(format!("pairee/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")
}

/// Downloads `url` into memory, at most `max` bytes. `Ok(None)` when the
/// server answers 404 (used for optional files such as signatures).
pub async fn fetch_limited(
    client: &reqwest::Client,
    url: &str,
    max: usize,
) -> Result<Option<Vec<u8>>> {
    let mut response = client.get(url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        anyhow::bail!("HTTP {} for {url}", response.status());
    }
    if response
        .content_length()
        .is_some_and(|len| len > max as u64)
    {
        anyhow::bail!("{url} is larger than the {max}-byte limit");
    }
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        push_limited(&mut data, &chunk, max).with_context(|| url.to_string())?;
    }
    Ok(Some(data))
}

/// Like [`fetch_limited`] but a 404 is an error.
pub async fn fetch_required(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>> {
    fetch_limited(client, url, max)
        .await?
        .ok_or_else(|| anyhow::anyhow!("{url} was not found (HTTP 404)"))
}

/// Appends `chunk` unless that makes `data` longer than `max`.
fn push_limited(data: &mut Vec<u8>, chunk: &[u8], max: usize) -> Result<()> {
    if data.len() + chunk.len() > max {
        anyhow::bail!("download exceeds the {max}-byte limit");
    }
    data.extend_from_slice(chunk);
    Ok(())
}

/// The text of a downloaded TOML/signature file.
pub fn utf8(bytes: Vec<u8>, what: &str) -> Result<String> {
    String::from_utf8(bytes).with_context(|| format!("{what} is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_past_the_limit_are_refused() {
        let mut data = Vec::new();
        push_limited(&mut data, &[0; 6], 10).unwrap();
        push_limited(&mut data, &[0; 4], 10).unwrap();
        assert!(push_limited(&mut data, &[0; 1], 10).is_err());
        assert_eq!(data.len(), 10);
    }
}
