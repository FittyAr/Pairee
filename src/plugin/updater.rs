use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PluginsLock {
    pub plugins: HashMap<String, PinnedPlugin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinnedPlugin {
    pub version: String,
    pub pinned: bool,
    pub files: HashMap<String, String>, // relative_path -> sha256
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryIndex {
    pub plugins: HashMap<String, RegistryPlugin>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryPlugin {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub languages: Option<Vec<String>>,
    pub hooks: Option<Vec<String>>,
    pub min_pairee: Option<String>,
    pub files: HashMap<String, String>, // relative_path -> sha256
}

fn get_lockfile_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("plugins.lock")
}

fn read_lockfile() -> PluginsLock {
    let path = get_lockfile_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(lock) = toml::from_str(&content) {
                return lock;
            }
        }
    }
    PluginsLock::default()
}

fn write_lockfile(lock: &PluginsLock) -> anyhow::Result<()> {
    let path = get_lockfile_path();
    let content = toml::to_string_pretty(lock)?;
    std::fs::write(&path, content)?;
    Ok(())
}

async fn fetch_index() -> anyhow::Result<RegistryIndex> {
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

pub fn list_installed() -> anyhow::Result<()> {
    let lock = read_lockfile();
    println!("Installed Plugins:");
    if lock.plugins.is_empty() {
        println!("  (none)");
        return Ok(());
    }
    for (name, info) in &lock.plugins {
        let pin_str = if info.pinned { " [PINNED]" } else { "" };
        println!("  - {} v{}{}", name, info.version, pin_str);
    }
    Ok(())
}

pub async fn search(query: &str) -> anyhow::Result<()> {
    println!("Searching registry for '{}'...", query);
    let index = fetch_index().await?;
    let query_lower = query.to_lowercase();

    for (name, plugin) in &index.plugins {
        if name.to_lowercase().contains(&query_lower)
            || plugin
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
        {
            let author = plugin.author.as_deref().unwrap_or("unknown");
            let lang_badges = plugin
                .languages
                .as_ref()
                .map(|langs| {
                    langs
                        .iter()
                        .map(|l| format!("[{}]", l.to_uppercase()))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            let hook_badge = if plugin
                .hooks
                .as_ref()
                .map(|h| !h.is_empty())
                .unwrap_or(false)
            {
                " [Hook]"
            } else {
                ""
            };

            println!(
                "* {} v{} by {}{}{}",
                plugin.name,
                plugin.version,
                author,
                hook_badge,
                if lang_badges.is_empty() {
                    "".to_string()
                } else {
                    format!(" {}", lang_badges)
                }
            );
            if let Some(ref desc) = plugin.description {
                println!("  Description: {}", desc);
            }
            println!();
        }
    }
    Ok(())
}

pub async fn show_info(name: &str) -> anyhow::Result<()> {
    let index = fetch_index().await?;
    let plugin = index
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    println!("Plugin: {}", plugin.name);
    println!("Version: {}", plugin.version);
    println!("Author: {}", plugin.author.as_deref().unwrap_or("unknown"));
    if let Some(ref desc) = plugin.description {
        println!("Description: {}", desc);
    }
    if let Some(ref min_p) = plugin.min_pairee {
        println!("Requires Pairee: >= {}", min_p);
    }
    if let Some(ref langs) = plugin.languages {
        println!("Supported languages: {}", langs.join(", "));
    }
    if let Some(ref hooks) = plugin.hooks {
        println!("Subscribes to hooks: {}", hooks.join(", "));
    }
    println!("Files:");
    for file in plugin.files.keys() {
        println!("  - {}", file);
    }
    Ok(())
}

pub async fn install(name: &str, version: Option<&str>) -> anyhow::Result<()> {
    let index = fetch_index().await?;
    let plugin = index
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    // Check version
    if let Some(ver) = version {
        if plugin.version != ver {
            anyhow::bail!(
                "Requested version '{}' does not match registry version '{}' (Registry only lists latest currently)",
                ver,
                plugin.version
            );
        }
    }

    let plugins_dir = crate::config::paths::get_config_dir()
        .join("plugins")
        .join(name);
    if !plugins_dir.exists() {
        std::fs::create_dir_all(&plugins_dir)?;
    }

    println!("Downloading {} v{}...", plugin.name, plugin.version);

    let client = reqwest::Client::builder().build()?;
    let mut downloaded_files = HashMap::new();

    for (rel_path, expected_hash) in &plugin.files {
        let file_url = format!(
            "https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/{}/{}",
            name, rel_path
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

pub fn remove(name: &str) -> anyhow::Result<()> {
    let mut lock = read_lockfile();
    if lock.plugins.remove(name).is_some() {
        let plugins_dir = crate::config::paths::get_config_dir()
            .join("plugins")
            .join(name);
        if plugins_dir.exists() {
            std::fs::remove_dir_all(plugins_dir)?;
        }
        write_lockfile(&lock)?;
        println!("Removed plugin '{}'.", name);
        Ok(())
    } else {
        anyhow::bail!("Plugin '{}' is not installed", name);
    }
}

pub fn pin(name: &str, pinned: bool) -> anyhow::Result<()> {
    let mut lock = read_lockfile();
    if let Some(plugin) = lock.plugins.get_mut(name) {
        plugin.pinned = pinned;
        write_lockfile(&lock)?;
        println!("Set pin status of plugin '{}' to {}.", name, pinned);
        Ok(())
    } else {
        anyhow::bail!("Plugin '{}' is not installed", name);
    }
}

pub fn verify() -> anyhow::Result<()> {
    let lock = read_lockfile();
    let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
    let mut clean = true;

    println!("Verifying installed plugins...");

    for (name, info) in &lock.plugins {
        println!("Plugin: {} v{}", name, info.version);
        let plugin_path = plugins_dir.join(name);

        for (rel_path, expected_hash) in &info.files {
            let file_path = plugin_path.join(rel_path);
            if !file_path.exists() {
                println!("  ✗ Missing file: {}", rel_path);
                clean = false;
                continue;
            }

            match crate::update::downloader::compute_sha256(&file_path) {
                Ok(actual_hash) => {
                    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                        println!(
                            "  ✗ Hash mismatch in {}: expected {}, got {}",
                            rel_path, expected_hash, actual_hash
                        );
                        clean = false;
                    } else {
                        println!("  ✓ {} verified.", rel_path);
                    }
                }
                Err(e) => {
                    println!("  ✗ Failed to calculate hash for {}: {:?}", rel_path, e);
                    clean = false;
                }
            }
        }
    }

    if clean {
        println!("All plugins verified successfully (integrity clean).");
        Ok(())
    } else {
        anyhow::bail!("Integrity verification failed for one or more plugins.")
    }
}
