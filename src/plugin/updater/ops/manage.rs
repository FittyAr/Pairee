//! Plugin management operations (update, remove, pin, verify).

use super::install::install;
use crate::plugin::updater::lockfile::{read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{fetch_blocklist, fetch_index};

pub async fn update(name: Option<&str>) -> anyhow::Result<()> {
    let blocklist = fetch_blocklist().await.unwrap_or_default();

    if let Some(n) = name {
        if let Some(reason) = blocklist.blocked.get(n) {
            anyhow::bail!(
                "Plugin '{}' cannot be updated because it is blocked: {}",
                n,
                reason
            );
        }
        let index = fetch_index().await?;
        let lock = read_lockfile();
        if let Some(info) = lock.plugins.get(n) {
            if info.pinned {
                anyhow::bail!(
                    "Plugin '{}' is pinned and cannot be updated. Unpin it first with 'pairee plugin unpin <name>'.",
                    n
                );
            }
            if let Some(reg_plugin) = index.plugins.get(n) {
                if reg_plugin.version == info.version {
                    println!("Plugin '{}' is already up to date (v{}).", n, info.version);
                    return Ok(());
                }
                install(n, None).await?;
            } else {
                anyhow::bail!("Plugin '{}' not found in registry.", n);
            }
        } else {
            anyhow::bail!("Plugin '{}' is not installed.", n);
        }
    } else {
        let index = fetch_index().await?;
        let lock = read_lockfile();
        let mut updated = 0;
        let mut plugins_to_update = Vec::new();

        // 1. Remove blocked plugins first
        for (n, info) in &lock.plugins {
            if let Some(reason) = blocklist.blocked.get(n) {
                println!(
                    "WARNING: Installed plugin '{}' has been BLOCKED by registry maintainers: {}. Automatically removing it for safety.",
                    n, reason
                );
                if let Err(e) = remove(n) {
                    tracing::error!(
                        "Failed to automatically remove blocked plugin '{}': {:?}",
                        n,
                        e
                    );
                }
                continue;
            }
            if info.pinned {
                println!("Skipping pinned plugin '{}'.", n);
                continue;
            }
            if let Some(reg_plugin) = index.plugins.get(n)
                && reg_plugin.version != info.version
            {
                plugins_to_update.push(n.clone());
            }
        }

        if plugins_to_update.is_empty() {
            println!("All plugins are up to date.");
            return Ok(());
        }

        for n in plugins_to_update {
            println!("Updating '{}'...", n);
            if let Err(e) = install(&n, None).await {
                tracing::error!("Failed to update plugin '{}': {:?}", n, e);
                println!("  ✗ Failed to update '{}': {:?}", n, e);
            } else {
                updated += 1;
            }
        }
        println!("Updated {} plugin(s).", updated);
    }
    Ok(())
}

pub fn remove(name: &str) -> anyhow::Result<()> {
    let mut lock = read_lockfile();
    if lock.plugins.remove(name).is_some() {
        let plugins_dir = crate::config::paths::get_config_dir()
            .join("plugins")
            .join(format!("{}.pairee", name));
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

pub async fn verify() -> anyhow::Result<()> {
    let lock = read_lockfile();
    let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
    let mut clean = true;

    println!("Verifying installed plugins...");

    let blocklist = fetch_blocklist().await.unwrap_or_default();

    for (name, info) in &lock.plugins {
        if let Some(reason) = blocklist.blocked.get(name) {
            println!(
                "  ✗ Plugin '{}' is BLOCKED by registry maintainers: {}",
                name, reason
            );
            clean = false;
        }

        println!("Plugin: {} v{}", name, info.version);
        let plugin_path = plugins_dir.join(format!("{}.pairee", name));

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
