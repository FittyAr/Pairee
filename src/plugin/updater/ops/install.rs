//! Plugin installation and download pipeline.

use super::install_guard::{safe_join, validate_identifier, verify_bytes_sha256};
use crate::plugin::updater::lockfile::{read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{
    fetch_blocklist, fetch_index, plugin_file_url, registry_author,
};
use crate::plugin::updater::types::{PinnedPlugin, RegistryPluginManifestWrapper};
use std::collections::HashMap;

pub async fn install(name: &str, version: Option<&str>) -> anyhow::Result<()> {
    validate_identifier("name", name)?;
    let blocklist = fetch_blocklist().await.unwrap_or_default();
    if let Some(reason) = blocklist.blocked.get(name) {
        anyhow::bail!(
            "Plugin '{}' is blocked and cannot be installed: {}",
            name,
            reason
        );
    }

    let index = fetch_index().await?;
    let plugin = index
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    // Check version
    if let Some(ver) = version
        && plugin.version != ver
    {
        anyhow::bail!(
            "Requested version '{}' does not match registry version '{}' (Registry only lists latest currently)",
            ver,
            plugin.version
        );
    }

    let author = registry_author(plugin.author.as_deref());
    validate_identifier("author", author)?;

    let plugins_dir = crate::config::paths::get_installed_plugin_dir(name);

    println!("Downloading {} v{}...", plugin.name, plugin.version);

    let client = reqwest::Client::builder().build()?;
    let manifest_url = plugin_file_url(author, name, "manifest.toml");
    let resp = client.get(&manifest_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to download plugin manifest: HTTP {}", resp.status());
    }
    let manifest_text = resp.text().await?;
    let manifest_wrapper: RegistryPluginManifestWrapper = toml::from_str(&manifest_text)?;
    let files = manifest_wrapper
        .files
        .ok_or_else(|| anyhow::anyhow!("Plugin manifest is missing [files] section"))?;

    // Reject hostile `[files]` keys before any network fetch or disk write.
    let mut targets = Vec::with_capacity(files.len());
    for (rel_path, expected_hash) in &files {
        targets.push((rel_path, expected_hash, safe_join(&plugins_dir, rel_path)?));
    }

    // Download and verify everything in memory first, so a tampered file
    // never reaches the plugin directory and no partial install is left.
    let mut verified = Vec::with_capacity(targets.len());
    for (rel_path, expected_hash, dest_path) in targets {
        let file_url = plugin_file_url(author, name, rel_path);
        let resp = client.get(&file_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "Failed to download file '{}': HTTP {}",
                rel_path,
                resp.status()
            );
        }
        let bytes = resp.bytes().await?;
        if let Err(e) = verify_bytes_sha256(&bytes, expected_hash) {
            anyhow::bail!("Verification failed for file '{}': {:?}", rel_path, e);
        }
        println!("  ✓ {} verified.", rel_path);
        verified.push((rel_path, expected_hash, dest_path, bytes));
    }

    let mut downloaded_files = HashMap::new();
    for (rel_path, expected_hash, dest_path, bytes) in verified {
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest_path, &bytes)?;
        downloaded_files.insert(rel_path.clone(), expected_hash.clone());
    }

    // Update lockfile
    let mut lock = read_lockfile();
    lock.plugins.insert(
        name.to_string(),
        PinnedPlugin {
            version: plugin.version.clone(),
            pinned: false,
            files: downloaded_files,
        },
    );
    write_lockfile(&lock)?;

    println!(
        "Successfully installed plugin '{}' v{}!",
        plugin.name, plugin.version
    );
    Ok(())
}
