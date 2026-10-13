//! Query operations for the plugin registry (listing, searching, showing
//! info). They return a [`Report`]; the CLI prints it.

use crate::config::settings::PluginConfig;
use crate::plugin::updater::http::{self, MAX_MANIFEST_BYTES};
use crate::plugin::updater::lockfile::read_lockfile;
use crate::plugin::updater::registry::{
    fetch_blocklist, fetch_index_with, fetch_signed, plugin_file_path, registry_author,
};
use crate::plugin::updater::report::Report;
use crate::plugin::updater::trust::RegistryTrust;
use crate::plugin::updater::types::{RegistryPlugin, RegistryPluginManifestWrapper};
use std::collections::HashMap;

/// Installed plugins with pin, trust, blocked and update badges. `plugins`
/// are the host's plugin settings (trust).
pub async fn list_installed(
    plugins: &HashMap<String, PluginConfig>,
    trust: &RegistryTrust,
) -> anyhow::Result<Report> {
    let lock = read_lockfile();
    let mut report = Report::default();
    report.line("Installed Plugins:");
    if lock.plugins.is_empty() {
        report.line("  (none)");
        return Ok(report);
    }
    let index = fetch_index_with(trust, &mut report).await.ok();
    let blocklist = fetch_blocklist(trust, &mut report).await?;

    for (name, info) in &lock.plugins {
        let pin_str = if info.pinned { " [PINNED]" } else { "" };
        let trusted_str = match plugins.get(name) {
            Some(p) if p.trusted => " [TRUSTED]",
            _ => " [UNTRUSTED]",
        };
        let disabled_str = match plugins.get(name) {
            Some(p) if !p.enabled => " [DISABLED]",
            _ => "",
        };
        let blocked = blocklist.blocked.contains_key(name);
        let blocked_str = if blocked { " [BLOCKED]" } else { "" };
        let update_str = index
            .as_ref()
            .filter(|_| !blocked)
            .and_then(|idx| idx.update_for(name, &info.version))
            .map(|v| format!(" (Update available: v{v})"))
            .unwrap_or_default();
        report.line(format!(
            "  - {name} v{}{pin_str}{trusted_str}{disabled_str}{blocked_str}{update_str}",
            info.version
        ));
    }
    Ok(report)
}

pub async fn check_updates(trust: &RegistryTrust) -> anyhow::Result<Report> {
    let mut report = Report::default();
    report.line("Checking for plugin updates...");
    let index = fetch_index_with(trust, &mut report).await?;
    let blocklist = fetch_blocklist(trust, &mut report).await?;
    let lock = read_lockfile();
    let mut updates_available = 0;

    for (name, info) in &lock.plugins {
        if let Some(reason) = blocklist.blocked.get(name) {
            report.line(format!(
                "  - {name}: v{} [BLOCKED] Reason: {reason}",
                info.version
            ));
            updates_available += 1;
            continue;
        }
        if let Some(latest) = index.update_for(name, &info.version) {
            let pin_str = if info.pinned {
                " [PINNED] (update skipped)"
            } else {
                ""
            };
            report.line(format!("  - {name}: {} -> {latest}{pin_str}", info.version));
            updates_available += 1;
        }
    }

    if updates_available == 0 {
        report.line("All plugins are up to date.");
    } else {
        report.line(format!(
            "Found {updates_available} plugin update(s). Run 'pairee plugin update' to update non-pinned plugins."
        ));
    }
    Ok(report)
}

/// `* name vX by author [Hook] [EN] [ES]` and its description.
fn describe(plugin: &RegistryPlugin, report: &mut Report) {
    let author = plugin.author.as_deref().unwrap_or("unknown");
    let lang_badges = plugin
        .languages
        .as_ref()
        .map(|langs| {
            langs
                .iter()
                .map(|l| format!(" [{}]", l.to_uppercase()))
                .collect::<String>()
        })
        .unwrap_or_default();
    let hook_badge = if plugin.hooks.as_ref().is_some_and(|h| !h.is_empty()) {
        " [Hook]"
    } else {
        ""
    };
    report.line(format!(
        "* {} v{} by {author}{hook_badge}{lang_badges}",
        plugin.name, plugin.version
    ));
    if let Some(ref desc) = plugin.description {
        report.line(format!("  Description: {desc}"));
    }
    report.line("");
}

pub async fn search(query: &str, trust: &RegistryTrust) -> anyhow::Result<Report> {
    let mut report = Report::default();
    report.line(format!("Searching registry for '{query}'..."));
    let index = fetch_index_with(trust, &mut report).await?;
    let blocklist = fetch_blocklist(trust, &mut report).await?;
    let query_lower = query.to_lowercase();

    for (name, plugin) in &index.plugins {
        let matches = name.to_lowercase().contains(&query_lower)
            || plugin
                .description
                .as_ref()
                .is_some_and(|d| d.to_lowercase().contains(&query_lower));
        if matches && !blocklist.blocked.contains_key(name) {
            describe(plugin, &mut report);
        }
    }
    Ok(report)
}

pub async fn show_info(name: &str, trust: &RegistryTrust) -> anyhow::Result<Report> {
    let mut report = Report::default();
    let blocklist = fetch_blocklist(trust, &mut report).await?;
    if let Some(reason) = blocklist.blocked.get(name) {
        anyhow::bail!("Plugin '{name}' is blocked by registry maintainers: {reason}");
    }

    let index = fetch_index_with(trust, &mut report).await?;
    let plugin = index
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    report.line(format!("Plugin: {}", plugin.name));
    report.line(format!("Version: {}", plugin.version));
    report.line(format!(
        "Author: {}",
        plugin.author.as_deref().unwrap_or("unknown")
    ));
    if let Some(ref desc) = plugin.description {
        report.line(format!("Description: {desc}"));
    }
    if let Some(ref min_p) = plugin.min_pairee {
        report.line(format!("Requires Pairee: >= {min_p}"));
    }
    if let Some(ref langs) = plugin.languages {
        report.line(format!("Supported languages: {}", langs.join(", ")));
    }
    if let Some(ref hooks) = plugin.hooks {
        report.line(format!("Subscribes to hooks: {}", hooks.join(", ")));
    }

    let author = registry_author(plugin.author.as_deref());
    let path = plugin_file_path(author, name, "manifest.toml");
    let client = http::client()?;
    if let Ok(Some(text)) =
        fetch_signed(&client, trust, &path, MAX_MANIFEST_BYTES, &mut report).await
        && let Ok(manifest) = toml::from_str::<RegistryPluginManifestWrapper>(&text)
        && let Some(files) = manifest.files
    {
        report.line("Files:");
        for file in files.keys() {
            report.line(format!("  - {file}"));
        }
    }
    Ok(report)
}
