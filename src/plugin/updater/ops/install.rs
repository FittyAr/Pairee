//! Plugin installation and download pipeline.

use super::files::{download_verified, remove_stale, write_all};
use super::install_guard::validate_identifier;
use crate::plugin::updater::http::{self, MAX_MANIFEST_BYTES};
use crate::plugin::updater::lockfile::{read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{
    fetch_blocklist, fetch_index_with, fetch_signed, plugin_file_path, registry_author,
};
use crate::plugin::updater::report::Report;
use crate::plugin::updater::trust::RegistryTrust;
use crate::plugin::updater::types::{PinnedPlugin, RegistryPluginManifestWrapper};

/// Installs (or updates) plugin `name` from the registry.
///
/// The index, the blocklist and the plugin's `manifest.toml` are checked
/// against their minisign signatures (see [`RegistryTrust`]); the manifest's
/// `[files]` hashes then pin every file. All downloads are size-limited and
/// verified in memory before anything is written; files the previous
/// version had and this one dropped are removed.
pub async fn install(
    name: &str,
    version: Option<&str>,
    trust: &RegistryTrust,
) -> anyhow::Result<Report> {
    let mut report = Report::default();
    validate_identifier("name", name)?;
    let blocklist = fetch_blocklist(trust, &mut report).await?;
    if let Some(reason) = blocklist.blocked.get(name) {
        anyhow::bail!("Plugin '{name}' is blocked and cannot be installed: {reason}");
    }

    let index = fetch_index_with(trust, &mut report).await?;
    let plugin = index
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

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
    let plugin_dir = crate::config::paths::get_installed_plugin_dir(name);
    report.line(format!(
        "Downloading {} v{}...",
        plugin.name, plugin.version
    ));

    let client = http::client()?;
    let manifest_path = plugin_file_path(author, name, "manifest.toml");
    let manifest_text = fetch_signed(
        &client,
        trust,
        &manifest_path,
        MAX_MANIFEST_BYTES,
        &mut report,
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("Plugin manifest {manifest_path} not found"))?;
    let manifest: RegistryPluginManifestWrapper = toml::from_str(&manifest_text)?;
    let files = manifest
        .files
        .ok_or_else(|| anyhow::anyhow!("Plugin manifest is missing [files] section"))?;

    let verified = download_verified(author, name, &files, &plugin_dir, &mut report).await?;
    let written = write_all(verified)?;

    let mut lock = read_lockfile();
    let previous = lock.plugins.remove(name);
    if let Some(old) = &previous {
        remove_stale(&plugin_dir, &old.files, &written, &mut report);
    }
    lock.plugins.insert(
        name.to_string(),
        PinnedPlugin {
            version: plugin.version.clone(),
            pinned: previous.is_some_and(|p| p.pinned),
            files: written,
        },
    );
    write_lockfile(&lock)?;

    report.line(format!(
        "Successfully installed plugin '{}' v{}!",
        plugin.name, plugin.version
    ));
    Ok(report)
}
