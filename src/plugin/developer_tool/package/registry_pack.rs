use super::super::{progress_progress, progress_status};
use super::registry_repo::fetch_or_clone_registry;
use super::validate::validate_for_publish_with_progress;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

pub fn package_to_registry(plugin_dir: &std::path::Path) -> anyhow::Result<String> {
    package_to_registry_with_progress(plugin_dir, None)
}

pub fn package_to_registry_with_progress(
    plugin_dir: &std::path::Path,
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<String> {
    // 1. Validate the plugin
    if let Err(err_msg) = validate_for_publish_with_progress(plugin_dir, progress.clone()) {
        anyhow::bail!(t("plugin_dev_validation_failed").replace("{}", &err_msg));
    }

    let manifest_path = plugin_dir.join("manifest.toml");
    progress_status(&progress, t("plugin_dev_progress_reading_manifest"));
    let content = std::fs::read_to_string(&manifest_path)?;
    let mut manifest = crate::plugin::loader::PluginManifest::parse(&content)?;
    let mut manifest_table: toml::Table = toml::from_str(&content)?;

    // Check for LICENSE file (case-insensitive)
    let mut license_file = None;
    if let Ok(entries) = std::fs::read_dir(plugin_dir) {
        for entry in entries.flatten() {
            let name_lower = entry.file_name().to_string_lossy().to_lowercase();
            if name_lower == "license" || name_lower == "license.txt" || name_lower == "license.md"
            {
                license_file = Some(entry.path());
                break;
            }
        }
    }

    let mut license_to_set = manifest.license.clone();

    if let Some(_path) = license_file {
        if manifest.license.is_none()
            || manifest
                .license
                .as_ref()
                .map(|l| l.trim().is_empty())
                .unwrap_or(true)
        {
            // Prompt the user for license name if stdin is a terminal
            let mut license_name = String::new();
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                println!("LICENSE file detected, but no license name specified in manifest.toml.");
                println!("Please enter the license name (e.g. MIT, GPL-3.0, Apache-2.0):");
                let _ = std::io::stdin().read_line(&mut license_name);
            }
            let license_name = license_name.trim().to_string();
            let license_name = if license_name.is_empty() {
                "Custom".to_string()
            } else {
                license_name
            };
            license_to_set = Some(license_name);
        }
    } else {
        // No license file present. Auto-assign MIT
        license_to_set = Some("MIT".to_string());
        let current_year = chrono::Local::now().format("%Y").to_string();
        let author_name = manifest.author.as_deref().unwrap_or("unknown");
        let mit_license_text = format!(
            "MIT License\n\nCopyright (c) {} {}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.\n",
            current_year, author_name
        );
        std::fs::write(plugin_dir.join("LICENSE"), mit_license_text)?;
    }

    if let Some(ref lic) = license_to_set {
        manifest.license = Some(lic.clone());
        if let Some(toml::Value::Table(plugin_table)) = manifest_table.get_mut("plugin") {
            plugin_table.insert("license".to_string(), toml::Value::String(lic.clone()));
        } else {
            manifest_table.insert("license".to_string(), toml::Value::String(lic.clone()));
        }
        let updated_content = toml::to_string_pretty(&manifest_table)?;
        std::fs::write(&manifest_path, updated_content)?;
    }

    let name = manifest.name.clone();

    // 2. Clone or update the registry repo in temporary directory
    let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
    progress_status(&progress, t("plugin_dev_progress_fetching_registry"));
    let _repo = fetch_or_clone_registry(&temp_dir)?;

    // 3. Copy plugin files to the cloned repo
    let author = manifest.author.as_deref().unwrap_or("unknown").trim();
    let author = if author.is_empty() { "unknown" } else { author };
    let first_char = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let first_char_str = if first_char.is_ascii_alphabetic() {
        first_char.to_string()
    } else {
        "_".to_string()
    };

    let dest_plugin_dir = temp_dir
        .join("registry")
        .join("plugins")
        .join(&first_char_str)
        .join(author)
        .join(&name);

    if dest_plugin_dir.exists() {
        let _ = std::fs::remove_dir_all(&dest_plugin_dir);
    }
    std::fs::create_dir_all(&dest_plugin_dir)?;

    let mut files_hash = HashMap::new();
    let files = crate::plugin::loader::get_plugin_files(plugin_dir);
    let total_files = files.len().max(1);
    progress_status(&progress, t("plugin_dev_progress_copying_files"));
    for (idx, (rel_path, src_file_path)) in files.into_iter().enumerate() {
        progress_progress(
            &progress,
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

    // Write sha256.sum inside the registry folder
    let mut sha_content = String::new();
    for (f, h) in &files_hash {
        sha_content.push_str(&format!("{}  {}\n", h, f));
    }
    std::fs::write(dest_plugin_dir.join("sha256.sum"), sha_content)?;

    // Copy manifest.toml to registry/plugins/.../manifest.toml with the [files] section appended
    let mut manifest_content = std::fs::read_to_string(&manifest_path)?;
    if !manifest_content.ends_with('\n') {
        manifest_content.push('\n');
    }
    manifest_content.push_str("\n[files]\n");
    for (f, h) in &files_hash {
        manifest_content.push_str(&format!("\"{}\" = \"{}\"\n", f, h));
    }
    std::fs::write(dest_plugin_dir.join("manifest.toml"), manifest_content)?;

    // 4. Update registry/index.toml
    progress_status(&progress, t("plugin_dev_progress_updating_index"));
    let index_path = temp_dir.join("registry").join("index.toml");
    let mut index_data = if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)?;
        toml::from_str::<crate::plugin::updater::RegistryIndex>(&content).unwrap_or_else(|_| {
            crate::plugin::updater::RegistryIndex {
                plugins: HashMap::new(),
            }
        })
    } else {
        std::fs::create_dir_all(index_path.parent().unwrap())?;
        crate::plugin::updater::RegistryIndex {
            plugins: HashMap::new(),
        }
    };

    // Construct RegistryPlugin
    let reg_plugin = crate::plugin::updater::RegistryPlugin {
        name: name.clone(),
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

    index_data.plugins.insert(name.clone(), reg_plugin);

    // Serialize and write back
    let serialized = toml::to_string_pretty(&index_data)?;
    std::fs::write(&index_path, serialized)?;

    let success_msg = t("plugin_dev_pack_success")
        .replace("{}", &name)
        .replace("{v}", &manifest.version);
    Ok(format!(
        "{}\nPath: {}",
        success_msg,
        dest_plugin_dir.display()
    ))
}
