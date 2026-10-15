//! Registry locations and the signed, size-limited fetches of its files.

use super::http::{self, MAX_INDEX_BYTES, MAX_MANIFEST_BYTES};
use super::report::Report;
use super::trust::{RegistryTrust, SignatureStatus};
use super::types::{Blocklist, RegistryIndex};
use anyhow::Result;

const REGISTRY_BASE_URL: &str =
    "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry";

/// Author used in registry paths: trimmed, `unknown` when missing or blank.
pub fn registry_author(author: Option<&str>) -> &str {
    match author.map(str::trim) {
        Some(a) if !a.is_empty() => a,
        _ => "unknown",
    }
}

/// Path of a plugin file inside `registry/`, sharded by the first
/// (lowercased) letter of the author, or `_` when it is not a letter. It is
/// also the name a signature's trusted comment must carry.
pub fn plugin_file_path(author: &str, name: &str, rel_path: &str) -> String {
    format!("{}/{rel_path}", plugin_dir_path(author, name))
}

/// `plugins/<shard>/<author>/<name>`: a plugin's folder inside `registry/`.
pub fn plugin_dir_path(author: &str, name: &str) -> String {
    let first = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let shard = if first.is_ascii_alphabetic() {
        first.to_string()
    } else {
        "_".to_string()
    };
    format!("plugins/{shard}/{author}/{name}")
}

/// URL of a file inside a plugin's registry folder.
pub fn plugin_file_url(author: &str, name: &str, rel_path: &str) -> String {
    registry_url(&plugin_file_path(author, name, rel_path))
}

fn registry_url(path: &str) -> String {
    format!("{REGISTRY_BASE_URL}/{path}")
}

pub fn parse_index(text: &str) -> Result<RegistryIndex> {
    Ok(toml::from_str(text)?)
}

/// A malformed blocklist is treated as empty rather than failing installs.
pub fn parse_blocklist(text: &str) -> Blocklist {
    toml::from_str(text).unwrap_or_default()
}

/// Downloads the registry file at `path` (at most `max` bytes) and checks
/// its `.minisig`. `Ok(None)` when the file itself does not exist; a bad
/// signature is an error, a missing one is recorded in `report`.
pub async fn fetch_signed(
    client: &reqwest::Client,
    trust: &RegistryTrust,
    path: &str,
    max: usize,
    report: &mut Report,
) -> Result<Option<String>> {
    let Some(data) = http::fetch_limited(client, &registry_url(path), max).await? else {
        return Ok(None);
    };
    check_signature(client, trust, path, &data, report).await?;
    http::utf8(data, path).map(Some)
}

/// Fetches `<path>.minisig` and checks `data` against it.
async fn check_signature(
    client: &reqwest::Client,
    trust: &RegistryTrust,
    path: &str,
    data: &[u8],
    report: &mut Report,
) -> Result<()> {
    let sig_url = format!("{}.minisig", registry_url(path));
    let sig = http::fetch_limited(client, &sig_url, MAX_MANIFEST_BYTES)
        .await?
        .map(|bytes| http::utf8(bytes, "signature"))
        .transpose()?;
    if trust.verify(data, sig.as_deref(), path)? == SignatureStatus::Unsigned {
        report.mark_unsigned(path);
    }
    Ok(())
}

/// The registry index (signature checked).
pub async fn fetch_index_with(trust: &RegistryTrust, report: &mut Report) -> Result<RegistryIndex> {
    let client = http::client()?;
    let text = fetch_signed(&client, trust, "index.toml", MAX_INDEX_BYTES, report)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch plugin registry: index.toml not found"))?;
    parse_index(&text)
}

/// The registry index; signature warnings are only logged.
pub async fn fetch_index(trust: &RegistryTrust) -> Result<RegistryIndex> {
    fetch_index_with(trust, &mut Report::default()).await
}

/// The blocklist. It fails **closed**: an unreachable blocklist is an error
/// (installing or updating without knowing what is blocked would defeat it),
/// and so is a missing one once the registry is known to be signed (see
/// [`RegistryTrust::expects_signed`]); only an unsigned registry may lack
/// the file, which then means "nothing blocked". An invalid signature is
/// always an error.
pub async fn fetch_blocklist(trust: &RegistryTrust, report: &mut Report) -> Result<Blocklist> {
    const PATH: &str = "blocklist.toml";
    let client = http::client()?;
    let fetched = http::fetch_limited(&client, &registry_url(PATH), MAX_INDEX_BYTES).await;
    let Some(data) = blocklist_bytes(fetched, trust.expects_signed())? else {
        return Ok(Blocklist::default());
    };
    check_signature(&client, trust, PATH, &data, report).await?;
    Ok(parse_blocklist(&http::utf8(data, PATH)?))
}

/// The downloaded blocklist, `None` when it legitimately does not exist.
fn blocklist_bytes(
    fetched: Result<Option<Vec<u8>>>,
    expects_signed: bool,
) -> Result<Option<Vec<u8>>> {
    match fetched {
        Ok(Some(data)) => Ok(Some(data)),
        Ok(None) if expects_signed => anyhow::bail!(
            "The plugin blocklist (blocklist.toml) is missing from a signed registry; refusing to continue without it"
        ),
        Ok(None) => Ok(None),
        Err(e) => {
            Err(e.context("The plugin blocklist is unreachable; refusing to continue without it"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreachable_or_missing_blocklist_in_a_signed_registry_is_an_error() {
        assert!(blocklist_bytes(Err(anyhow::anyhow!("offline")), false).is_err());
        assert!(blocklist_bytes(Ok(None), true).is_err());
        assert_eq!(blocklist_bytes(Ok(None), false).unwrap(), None);
        assert_eq!(
            blocklist_bytes(Ok(Some(vec![1])), true).unwrap(),
            Some(vec![1])
        );
    }
}
