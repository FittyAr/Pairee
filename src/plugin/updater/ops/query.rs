//! Query operations for the plugin registry (listing, searching, showing info).

use crate::plugin::updater::lockfile::read_lockfile;
use crate::plugin::updater::registry::{fetch_blocklist, fetch_index};
use crate::plugin::updater::types::RegistryPluginManifestWrapper;

pub async fn list_installed() -> anyhow::Result<()> {
    let lock = read_lockfile();
    println!("Installed Plugins:");
    if lock.plugins.is_empty() {
        println!("  (none)");
        return Ok(());
    }

    let index = fetch_index().await.ok();
    let blocklist = fetch_blocklist().await.unwrap_or_default();
    let config = crate::config::AppConfig::load_or_create().ok();

    for (name, info) in &lock.plugins {
        let pin_str = if info.pinned { " [PINNED]" } else { "" };
        let trusted_str = if let Some(ref conf) = config {
            let trusted = conf
                .settings
                .plugins
                .get(name)
                .map(|p| p.trusted)
                .unwrap_or(false);
            if trusted {
                " [TRUSTED]"
            } else {
                " [UNTRUSTED]"
            }
        } else {
            " [UNTRUSTED]"
        };

        let blocked_str = if blocklist.blocked.contains_key(name) {
            " [BLOCKED]"
        } else {
            ""
        };

        let update_str = if blocked_str.is_empty() {
            if let Some(ref idx) = index {
                if let Some(reg_plugin) = idx.plugins.get(name) {
                    if reg_plugin.version != info.version {
                        format!(" (Update available: v{})", reg_plugin.version)
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        println!(
            "  - {} v{}{}{}{}{}",
            name, info.version, pin_str, trusted_str, blocked_str, update_str
        );
    }
    Ok(())
}

pub async fn check_updates() -> anyhow::Result<()> {
    println!("Checking for plugin updates...");
    let index = fetch_index().await?;
    let blocklist = fetch_blocklist().await.unwrap_or_default();
    let lock = read_lockfile();
    let mut updates_available = 0;

    for (name, info) in &lock.plugins {
        if let Some(reason) = blocklist.blocked.get(name) {
            println!(
                "  - {}: v{} [BLOCKED] Reason: {}",
                name, info.version, reason
            );
            updates_available += 1;
            continue;
        }
        if let Some(reg_plugin) = index.plugins.get(name)
            && reg_plugin.version != info.version
        {
            let pin_str = if info.pinned {
                " [PINNED] (update skipped)"
            } else {
                ""
            };
            println!(
                "  - {}: {} -> {}{}",
                name, info.version, reg_plugin.version, pin_str
            );
            updates_available += 1;
        }
    }

    if updates_available == 0 {
        println!("All plugins are up to date.");
    } else {
        println!(
            "Found {} plugin update(s). Run 'pairee plugin update' to update non-pinned plugins.",
            updates_available
        );
    }
    Ok(())
}

pub async fn search(query: &str) -> anyhow::Result<()> {
    println!("Searching registry for '{}'...", query);
    let index = fetch_index().await?;
    let blocklist = fetch_blocklist().await.unwrap_or_default();
    let query_lower = query.to_lowercase();

    for (name, plugin) in &index.plugins {
        if blocklist.blocked.contains_key(name) {
            continue;
        }
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
    let blocklist = fetch_blocklist().await.unwrap_or_default();
    if let Some(reason) = blocklist.blocked.get(name) {
        anyhow::bail!(
            "Plugin '{}' is blocked by registry maintainers: {}",
            name,
            reason
        );
    }

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
    if let Ok(resp) = client.get(&manifest_url).send().await
        && resp.status().is_success()
        && let Ok(text) = resp.text().await
        && let Ok(manifest_wrapper) = toml::from_str::<RegistryPluginManifestWrapper>(&text)
        && let Some(files) = manifest_wrapper.files
    {
        println!("Files:");
        for file in files.keys() {
            println!("  - {}", file);
        }
    }
    Ok(())
}
