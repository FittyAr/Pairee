use super::types::{Blocklist, RegistryIndex};

pub async fn fetch_index() -> anyhow::Result<RegistryIndex> {
    let url =
        "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/index.toml";
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let text = resp.text().await?;
        let index: RegistryIndex = toml::from_str(&text)?;
        Ok(index)
    } else {
        anyhow::bail!("Failed to fetch plugin registry: HTTP {}", resp.status());
    }
}

pub async fn fetch_blocklist() -> anyhow::Result<Blocklist> {
    let url =
        "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/blocklist.toml";
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let text = resp.text().await?;
        let blocklist: Blocklist = toml::from_str(&text).unwrap_or_default();
        Ok(blocklist)
    } else {
        Ok(Blocklist::default())
    }
}
