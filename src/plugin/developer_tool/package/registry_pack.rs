use super::super::{progress_progress, progress_status};
use super::registry_repo::fetch_or_clone_registry;
use super::validate::validate_for_publish_with_progress;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use crate::plugin::loader::PluginManifest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

pub fn package_to_registry(plugin_dir: &Path) -> anyhow::Result<String> {
    package_to_registry_with_progress(plugin_dir, None)
}

pub fn package_to_registry_with_progress(
    plugin_dir: &Path,
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<String> {
    // 1. Validate the plugin
    if let Err(err_msg) = validate_for_publish_with_progress(plugin_dir, progress.clone()) {
        anyhow::bail!(t("plugin_dev_validation_failed").replace("{}", &err_msg));
    }

    let manifest_path = plugin_dir.join("manifest.toml");
    progress_status(&progress, t("plugin_dev_progress_reading_manifest"));
    let content = std::fs::read_to_string(&manifest_path)?;
    let mut manifest = PluginManifest::parse(&content)?;
    if let Some(lic) = super::license::resolve_license(plugin_dir, &manifest)? {
        write_manifest_license(&manifest_path, &content, &lic)?;
        manifest.license = Some(lic);
    }

    // 2. Clone or update the registry repo in temporary directory
    let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
    progress_status(&progress, t("plugin_dev_progress_fetching_registry"));
    let _repo = fetch_or_clone_registry(&temp_dir)?;

    // 3. Copy plugin files to the cloned repo
    let dest_plugin_dir = registry_plugin_dir(&temp_dir, &manifest);
    if dest_plugin_dir.exists() {
        let _ = std::fs::remove_dir_all(&dest_plugin_dir);
    }
    std::fs::create_dir_all(&dest_plugin_dir)?;
    let files_hash = copy_plugin_files(plugin_dir, &dest_plugin_dir, &progress)?;
    write_checksums(&manifest_path, &dest_plugin_dir, &files_hash)?;

    // 4. Update registry/index.toml
    progress_status(&progress, t("plugin_dev_progress_updating_index"));
    update_index(&temp_dir.join("registry").join("index.toml"), &manifest)?;

    let success_msg = t("plugin_dev_pack_success")
        .replace("{}", &manifest.name)
        .replace("{v}", &manifest.version);
    Ok(format!(
        "{}\nPath: {}",
        success_msg,
        dest_plugin_dir.display()
    ))
}

/// Sets `license` in `[plugin]` (or at the top level) of `manifest.toml`.
fn write_manifest_license(manifest_path: &Path, content: &str, lic: &str) -> anyhow::Result<()> {
    let mut manifest_table: toml::Table = toml::from_str(content)?;
    let value = toml::Value::String(lic.to_string());
    if let Some(toml::Value::Table(plugin_table)) = manifest_table.get_mut("plugin") {
        plugin_table.insert("license".to_string(), value);
    } else {
        manifest_table.insert("license".to_string(), value);
    }
    std::fs::write(manifest_path, toml::to_string_pretty(&manifest_table)?)?;
    Ok(())
}

/// `registry/plugins/<initial>/<author>/<name>` inside the registry clone.
fn registry_plugin_dir(temp_dir: &Path, manifest: &PluginManifest) -> PathBuf {
    let author = manifest.author.as_deref().unwrap_or("unknown").trim();
    let author = if author.is_empty() { "unknown" } else { author };
    let first_char = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let first_char_str = if first_char.is_ascii_alphabetic() {
        first_char.to_string()
    } else {
        "_".to_string()
    };
    temp_dir
        .join("registry")
        .join("plugins")
        .join(&first_char_str)
        .join(author)
        .join(&manifest.name)
}

/// Copies the plugin files, returning the SHA-256 of each relative path.
fn copy_plugin_files(
    plugin_dir: &Path,
    dest_plugin_dir: &Path,
    progress: &Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<HashMap<String, String>> {
    let mut files_hash = HashMap::new();
    let files = crate::plugin::loader::get_plugin_files(plugin_dir);
    let total_files = files.len().max(1);
    progress_status(progress, t("plugin_dev_progress_copying_files"));
    for (idx, (rel_path, src_file_path)) in files.into_iter().enumerate() {
        progress_progress(
            progress,
            t("plugin_dev_progress_copying_file")
                .replace("{}", &rel_path)
                .replace("{n}", &(idx + 1).to_string())
                .replace("{t}", &total_files.to_string()),
            idx + 1,
            total_files,
        );
        let dest_file_path = dest_plugin_dir.join(&rel_path);
        if let Some(parent) = dest_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src_file_path, &dest_file_path)?;

        let hash = crate::update::downloader::compute_sha256(&dest_file_path)?;
        files_hash.insert(rel_path, hash);
    }
    Ok(files_hash)
}

/// Writes `sha256.sum` and the manifest with a `[files]` checksum section.
fn write_checksums(
    manifest_path: &Path,
    dest_plugin_dir: &Path,
    files_hash: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let mut sha_content = String::new();
    for (f, h) in files_hash {
        sha_content.push_str(&format!("{}  {}\n", h, f));
    }
    std::fs::write(dest_plugin_dir.join("sha256.sum"), sha_content)?;

    let mut manifest_content = std::fs::read_to_string(manifest_path)?;
    if !manifest_content.ends_with('\n') {
        manifest_content.push('\n');
    }
    manifest_content.push_str("\n[files]\n");
    for (f, h) in files_hash {
        manifest_content.push_str(&format!("\"{}\" = \"{}\"\n", f, h));
    }
    std::fs::write(dest_plugin_dir.join("manifest.toml"), manifest_content)?;
    Ok(())
}

/// Adds or replaces the plugin entry of `registry/index.toml`.
fn update_index(index_path: &Path, manifest: &PluginManifest) -> anyhow::Result<()> {
    use crate::plugin::updater::{RegistryIndex, RegistryPlugin};
    let empty = || RegistryIndex {
        plugins: HashMap::new(),
    };
    let mut index_data = if index_path.exists() {
        let content = std::fs::read_to_string(index_path)?;
        toml::from_str::<RegistryIndex>(&content).unwrap_or_else(|_| empty())
    } else {
        std::fs::create_dir_all(index_path.parent().unwrap())?;
        empty()
    };

    let reg_plugin = RegistryPlugin {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        languages: manifest.languages.clone(),
        hooks: manifest
            .keybindings
            .as_ref()
            .map(|kb| kb.values().cloned().collect()),
        min_pairee: manifest.min_pairee.clone(),
    };
    index_data.plugins.insert(manifest.name.clone(), reg_plugin);

    std::fs::write(index_path, toml::to_string_pretty(&index_data)?)?;
    Ok(())
}
