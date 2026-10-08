use super::types::{Blocklist, RegistryIndex};

const REGISTRY_BASE_URL: &str =
    "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry";

/// Author used in registry paths: trimmed, `unknown` when missing or blank.
pub fn registry_author(author: Option<&str>) -> &str {
    match author.map(str::trim) {
        Some(a) if !a.is_empty() => a,
        _ => "unknown",
    }
}

/// URL of a file inside a plugin's registry folder, sharded by the first
/// (lowercased) letter of the author, or `_` when it is not a letter.
pub fn plugin_file_url(author: &str, name: &str, rel_path: &str) -> String {
    let first = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let shard = if first.is_ascii_alphabetic() {
        first.to_string()
    } else {
        "_".to_string()
    };
    format!("{REGISTRY_BASE_URL}/plugins/{shard}/{author}/{name}/{rel_path}")
}

pub fn parse_index(text: &str) -> anyhow::Result<RegistryIndex> {
    Ok(toml::from_str(text)?)
}

/// A malformed blocklist is treated as empty rather than failing installs.
pub fn parse_blocklist(text: &str) -> Blocklist {
    toml::from_str(text).unwrap_or_default()
}

pub async fn fetch_index() -> anyhow::Result<RegistryIndex> {
    let url = format!("{REGISTRY_BASE_URL}/index.toml");
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(&url).send().await?;
    if resp.status().is_success() {
        let text = resp.text().await?;
        parse_index(&text)
    } else {
        anyhow::bail!("Failed to fetch plugin registry: HTTP {}", resp.status());
    }
}

pub async fn fetch_blocklist() -> anyhow::Result<Blocklist> {
    let url = format!("{REGISTRY_BASE_URL}/blocklist.toml");
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(&url).send().await?;
    if resp.status().is_success() {
        let text = resp.text().await?;
        Ok(parse_blocklist(&text))
    } else {
        Ok(Blocklist::default())
    }
}
