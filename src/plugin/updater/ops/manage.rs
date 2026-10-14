//! Plugin management operations (update, remove, pin, verify). They return
//! a [`Report`] instead of printing.

use super::install::install;
use super::install_guard::{safe_join, validate_identifier};
use crate::plugin::updater::lockfile::{read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{fetch_blocklist, fetch_index_with};
use crate::plugin::updater::report::Report;
use crate::plugin::updater::trust::RegistryTrust;

/// Updates `name`, or every non-pinned plugin with a newer registry
/// version (removing blocked ones first) when `name` is `None`.
pub async fn update(name: Option<&str>, trust: &RegistryTrust) -> anyhow::Result<Report> {
    let mut report = Report::default();
    let blocklist = fetch_blocklist(trust, &mut report).await?;
    let index = fetch_index_with(trust, &mut report).await?;
    let lock = read_lockfile();

    if let Some(n) = name {
        if let Some(reason) = blocklist.blocked.get(n) {
            anyhow::bail!("Plugin '{n}' cannot be updated because it is blocked: {reason}");
        }
        let info = lock
            .plugins
            .get(n)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{n}' is not installed."))?;
        if info.pinned {
            anyhow::bail!(
                "Plugin '{n}' is pinned and cannot be updated. Unpin it first with 'pairee plugin unpin <name>'."
            );
        }
        let reg_plugin = index
            .plugins
            .get(n)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{n}' not found in registry."))?;
        if reg_plugin.version == info.version {
            report.line(format!(
                "Plugin '{n}' is already up to date (v{}).",
                info.version
            ));
        } else {
            report.absorb(install(n, None, trust).await?);
        }
        return Ok(report);
    }

    let mut to_update = Vec::new();
    for (n, info) in &lock.plugins {
        if let Some(reason) = blocklist.blocked.get(n) {
            report.warn(format!(
                "Installed plugin '{n}' has been BLOCKED by registry maintainers: {reason}. Removing it for safety."
            ));
            if let Err(e) = remove(n) {
                report.warn(format!("Failed to remove blocked plugin '{n}': {e:#}"));
            }
            continue;
        }
        if info.pinned {
            report.line(format!("Skipping pinned plugin '{n}'."));
            continue;
        }
        if index.update_for(n, &info.version).is_some() {
            to_update.push(n.clone());
        }
    }
    if to_update.is_empty() {
        report.line("All plugins are up to date.");
        return Ok(report);
    }
    let mut updated = 0;
    for n in to_update {
        report.line(format!("Updating '{n}'..."));
        match install(&n, None, trust).await {
            Ok(r) => {
                report.absorb(r);
                updated += 1;
            }
            Err(e) => report.warn(format!("Failed to update '{n}': {e:#}")),
        }
    }
    report.line(format!("Updated {updated} plugin(s)."));
    Ok(report)
}

pub fn remove(name: &str) -> anyhow::Result<Report> {
    validate_identifier("name", name)?;
    let mut lock = read_lockfile();
    if lock.plugins.remove(name).is_none() {
        anyhow::bail!("Plugin '{}' is not installed", name);
    }
    let plugins_dir = crate::config::paths::get_installed_plugin_dir(name);
    if plugins_dir.exists() {
        std::fs::remove_dir_all(plugins_dir)?;
    }
    write_lockfile(&lock)?;
    let mut report = Report::default();
    report.line(format!("Removed plugin '{name}'."));
    Ok(report)
}

pub fn pin(name: &str, pinned: bool) -> anyhow::Result<Report> {
    let mut lock = read_lockfile();
    let plugin = lock
        .plugins
        .get_mut(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' is not installed", name))?;
    plugin.pinned = pinned;
    write_lockfile(&lock)?;
    let mut report = Report::default();
    report.line(format!("Set pin status of plugin '{name}' to {pinned}."));
    Ok(report)
}

/// Re-hashes every installed file against the lockfile. Fails (with the
/// full report in the error) when anything is missing, changed or blocked.
pub async fn verify(trust: &RegistryTrust) -> anyhow::Result<Report> {
    let lock = read_lockfile();
    let mut report = Report::default();
    let mut clean = true;
    report.line("Verifying installed plugins...");
    let blocklist = fetch_blocklist(trust, &mut report).await?;

    for (name, info) in &lock.plugins {
        if let Some(reason) = blocklist.blocked.get(name) {
            report.line(format!(
                "  ✗ Plugin '{name}' is BLOCKED by registry maintainers: {reason}"
            ));
            clean = false;
        }
        report.line(format!("Plugin: {name} v{}", info.version));
        let plugin_path = crate::config::paths::get_installed_plugin_dir(name);
        for (rel_path, expected_hash) in &info.files {
            let Ok(file_path) = safe_join(&plugin_path, rel_path) else {
                clean = false;
                report.line(format!("  ✗ Unsafe path in the lockfile: {rel_path}"));
                continue;
            };
            let line = match crate::update::downloader::compute_sha256(&file_path) {
                Ok(actual) if actual.eq_ignore_ascii_case(expected_hash) => {
                    format!("  ✓ {rel_path} verified.")
                }
                Ok(actual) => {
                    clean = false;
                    format!(
                        "  ✗ Hash mismatch in {rel_path}: expected {expected_hash}, got {actual}"
                    )
                }
                Err(_) if !file_path.exists() => {
                    clean = false;
                    format!("  ✗ Missing file: {rel_path}")
                }
                Err(e) => {
                    clean = false;
                    format!("  ✗ Failed to calculate hash for {rel_path}: {e:#}")
                }
            };
            report.line(line);
        }
    }

    if clean {
        report.line("All plugins verified successfully (integrity clean).");
        Ok(report)
    } else {
        anyhow::bail!("{report}\nIntegrity verification failed for one or more plugins.")
    }
}
