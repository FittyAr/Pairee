//! Plugin installation and download pipeline.

use crate::plugin::updater::lockfile::{read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{fetch_blocklist, fetch_index};
use crate::plugin::updater::types::{PinnedPlugin, RegistryPluginManifestWrapper};
use std::collections::HashMap;

pub async fn install(name: &str, version: Option<&str>) -> anyhow::Result<()> {
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

    let plugins_dir = crate::config::paths::get_config_dir()
        .join("plugins")
        .join(format!("{}.pairee", name));
    if !plugins_dir.exists() {
        std::fs::create_dir_all(&plugins_dir)?;
    }

    println!("Downloading {} v{}...", plugin.name, plugin.version);

    let author = plugin.author.as_deref().unwrap_or("unknown").trim();
    let author = if author.is_empty() { "unknown" } else { author };
    let first_char = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let first_char_str = if first_char.is_ascii_alphabetic() {
        first_char.to_string()
    } else {
        "_".to_string()
    };

    let client = reqwest::Client::builder().build()?;
    let manifest_url = format!(
        "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/plugins/{}/{}/{}/manifest.toml",
        first_char_str, author, name
    );
    let resp = client.get(&manifest_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to download plugin manifest: HTTP {}", resp.status());
    }
    let manifest_text = resp.text().await?;
    let manifest_wrapper: RegistryPluginManifestWrapper = toml::from_str(&manifest_text)?;
    let files = manifest_wrapper
        .files
        .ok_or_else(|| anyhow::anyhow!("Plugin manifest is missing [files] section"))?;

    let mut downloaded_files = HashMap::new();

    for (rel_path, expected_hash) in &files {
        let file_url = format!(
            "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/plugins/{}/{}/{}/{}",
            first_char_str, author, name, rel_path
        );
        let dest_path = plugins_dir.join(rel_path);

        // Ensure subdirectories exist
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let resp = client.get(&file_url).send().await?;
        if !resp.status().is_success() {
            // Clean up downloaded files
            let _ = std::fs::remove_dir_all(&plugins_dir);
            anyhow::bail!(
                "Failed to download file '{}': HTTP {}",
                rel_path,
                resp.status()
            );
        }

        let bytes = resp.bytes().await?;
        std::fs::write(&dest_path, &bytes)?;

        // Verify SHA-256
        if let Err(e) = crate::update::downloader::verify_sha256(&dest_path, expected_hash) {
            let _ = std::fs::remove_dir_all(&plugins_dir);
            anyhow::bail!("Verification failed for file '{}': {:?}", rel_path, e);
        }

        downloaded_files.insert(rel_path.clone(), expected_hash.clone());
        println!("  ✓ {} verified.", rel_path);
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
