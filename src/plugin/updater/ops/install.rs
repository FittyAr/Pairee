//! Plugin installation and download pipeline.

use super::files::{VerifiedFile, download_verified, install_staged};
use super::install_guard::validate_identifier;
use crate::plugin::updater::http::{self, MAX_MANIFEST_BYTES};
use crate::plugin::updater::lockfile::{try_read_lockfile, write_lockfile};
use crate::plugin::updater::registry::{
    fetch_blocklist, fetch_index_with, fetch_signed, plugin_file_path, registry_author,
};
use crate::plugin::updater::report::Report;
use crate::plugin::updater::trust::RegistryTrust;
use crate::plugin::updater::types::{PinnedPlugin, RegistryPluginManifestWrapper};

/// The manifest's own file name: installed from the signed text, never
/// from a `[files]` entry (a file cannot list its own hash).
const MANIFEST_FILE: &str = "manifest.toml";

/// Installs (or updates) plugin `name` from the registry.
///
/// The index, the blocklist and the plugin's `manifest.toml` are checked
/// against their minisign signatures (see [`RegistryTrust`]); the manifest's
/// `[files]` hashes then pin every file. All downloads are size-limited and
/// verified in memory before anything is written, the plugin's
/// `min_pairee` and `@since` are checked against this Pairee, and the new
/// folder is staged and swapped in atomically (see [`install_staged`]);
/// files the previous version had and this one dropped are removed.
pub async fn install(
    name: &str,
    version: Option<&str>,
    trust: &RegistryTrust,
) -> anyhow::Result<Report> {
    let mut report = Report::default();
    validate_identifier("name", name)?;
    // Before anything is fetched or written: a corrupt lockfile must stop
    // the install, not be replaced by an empty one.
    try_read_lockfile()?;
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
    // The signed manifest names the plugin and version it belongs to, so
    // an old signed manifest cannot be served for the version the index
    // lists.
    let meta = crate::plugin::loader::PluginManifest::parse(&manifest_text)?;
    if meta.name != name || meta.version != plugin.version {
        anyhow::bail!(
            "Plugin manifest is for '{}' v{}, but the registry lists '{name}' v{}",
            meta.name,
            meta.version,
            plugin.version
        );
    }
    ensure_compatible(&meta, plugin.min_pairee.as_deref(), name)?;
    let manifest: RegistryPluginManifestWrapper = toml::from_str(&manifest_text)?;
    let mut files = manifest
        .files
        .ok_or_else(|| anyhow::anyhow!("Plugin manifest is missing [files] section"))?;
    files.remove(MANIFEST_FILE);

    let mut verified = download_verified(author, name, &files, &plugin_dir, &mut report).await?;
    if let Some(main) = verified.iter().find(|f| f.rel_path == "main.lua") {
        ensure_api_supported(&main.bytes, name)?;
    }
    verified.push(VerifiedFile::new(
        &plugin_dir,
        MANIFEST_FILE,
        manifest_text.into_bytes(),
    )?);
    // Read again: pins may have changed while the files downloaded.
    let mut lock = try_read_lockfile()?;
    let previous = lock.plugins.remove(name);
    let written = install_staged(
        &plugin_dir,
        verified,
        previous.as_ref().map(|old| &old.files),
        &mut report,
    )?;
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

/// Refuses a plugin that needs a newer Pairee than this one (`min_pairee`
/// of the signed manifest or of the index): it would be installed, then fail
/// to load.
fn ensure_compatible(
    meta: &crate::plugin::loader::PluginManifest,
    index_min: Option<&str>,
    name: &str,
) -> anyhow::Result<()> {
    for min in [meta.min_pairee.as_deref(), index_min]
        .into_iter()
        .flatten()
    {
        crate::plugin::loader::check_version_compatibility(min, env!("CARGO_PKG_VERSION"))
            .map_err(|e| anyhow::anyhow!("Plugin '{name}' cannot be installed: {e}"))?;
    }
    Ok(())
}

/// Refuses a plugin whose `--- @since` needs a newer plugin API than this
/// Pairee provides.
fn ensure_api_supported(main_lua: &[u8], name: &str) -> anyhow::Result<()> {
    let source = String::from_utf8_lossy(main_lua);
    crate::plugin::annotations::Annotations::parse(&source)
        .check_since(crate::plugin::runtime::api_version::LUA_API_VERSION)
        .map_err(|e| anyhow::anyhow!("Plugin '{name}' cannot be installed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(min: Option<&str>) -> crate::plugin::loader::PluginManifest {
        let min = min
            .map(|m| format!("min_pairee = {m:?}\n"))
            .unwrap_or_default();
        crate::plugin::loader::PluginManifest::parse(&format!(
            "name = \"demo\"\nversion = \"1.0.0\"\n{min}"
        ))
        .unwrap()
    }

    #[test]
    fn a_plugin_needing_a_newer_pairee_is_refused_at_install() {
        assert!(ensure_compatible(&meta(None), None, "demo").is_ok());
        assert!(ensure_compatible(&meta(Some("0.1.0")), Some("0.1"), "demo").is_ok());
        assert!(ensure_compatible(&meta(Some("999.0.0")), None, "demo").is_err());
        assert!(ensure_compatible(&meta(None), Some("999.0"), "demo").is_err());
    }

    #[test]
    fn a_plugin_needing_a_newer_api_is_refused_at_install() {
        assert!(ensure_api_supported(b"return {}", "demo").is_ok());
        let err = ensure_api_supported(b"--- @since 99.0\nreturn {}", "demo").unwrap_err();
        assert!(err.to_string().contains("@since 99.0"), "{err}");
    }
}
