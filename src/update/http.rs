//! HTTP plumbing shared by the self-updater and the plugin registry client:
//! one client configuration and one bounded body reader, so a hostile or
//! broken server can never make Pairee buffer an unbounded response.

use anyhow::{Context as _, Result};
use std::time::Duration;

/// HTTP client with Pairee's user agent and a whole-request `timeout`.
pub fn client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(concat!("pairee/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")
}

/// Reads the body of `response` into memory, refusing it as soon as it
/// grows past `max` bytes (and before reading when the declared length is
/// already larger). `on_progress(received, total)` runs after every chunk;
/// `total` is 0 when the server did not declare a length.
pub async fn read_limited(
    mut response: reqwest::Response,
    max: usize,
    mut on_progress: impl FnMut(usize, u64),
) -> Result<Vec<u8>> {
    let total = response.content_length().unwrap_or(0);
    if total > max as u64 {
        anyhow::bail!("response of {total} bytes is larger than the {max}-byte limit");
    }
    let mut data = Vec::with_capacity(total as usize);
    while let Some(chunk) = response.chunk().await.context("stream error")? {
        push_limited(&mut data, &chunk, max)?;
        on_progress(data.len(), total);
    }
    Ok(data)
}

/// Appends `chunk` unless that makes `data` longer than `max`.
fn push_limited(data: &mut Vec<u8>, chunk: &[u8], max: usize) -> Result<()> {
    if data.len() + chunk.len() > max {
        anyhow::bail!("download exceeds the {max}-byte limit");
    }
    data.extend_from_slice(chunk);
    Ok(())
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
