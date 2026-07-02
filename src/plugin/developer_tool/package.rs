use super::find_pairee_repo;
use crate::config::localization::t;
use std::collections::HashMap;

pub fn package() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let msg = package_to_registry(&path)?;
    println!("{}", msg);
    Ok(())
}

pub fn validate_for_publish(path: &std::path::Path) -> Result<(), String> {
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(t("plugin_dev_submit_no_manifest"));
    }

    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(t("plugin_dev_err_read_manifest").replace("{:?}", &format!("{:?}", e)));
        }
    };

    let manifest = match crate::plugin::loader::PluginManifest::parse(&content) {
        Ok(m) => m,
        Err(e) => {
            return Err(t("plugin_dev_err_parse_manifest").replace("{:?}", &format!("{:?}", e)));
        }
    };

    // 1. Validate Icon
    let icon_rel = match &manifest.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => return Err(t("plugin_dev_publish_no_icon")),
    };
    let icon_path = path.join(icon_rel);
    if !icon_path.exists() || !icon_path.is_file() {
        return Err(t("plugin_dev_publish_no_icon"));
    }

    // Check dimensions: 256x256 or 512x512
    match image::image_dimensions(&icon_path) {
        Ok((w, h)) => {
            if (w != 256 || h != 256) && (w != 512 || h != 512) {
                return Err(t("plugin_dev_publish_icon_invalid_size")
                    .replace("{w}", &w.to_string())
                    .replace("{h}", &h.to_string()));
            }
        }
        Err(e) => {
            return Err(
                t("plugin_dev_publish_icon_invalid_format").replace("{:?}", &format!("{:?}", e))
            );
        }
    }

    // 2. Validate Screenshots
    let screenshots = match &manifest.screenshots {
        Some(s) if !s.is_empty() => s,
        _ => return Err(t("plugin_dev_publish_no_screenshots")),
    };

    for scr_rel in screenshots {
        if scr_rel.trim().is_empty() {
            continue;
        }
        let scr_path = path.join(scr_rel);
        if !scr_path.exists() || !scr_path.is_file() {
            return Err(t("plugin_dev_publish_screenshot_not_found").replace("{}", scr_rel));
        }

        // Validate screenshot size: minimum 640x480 pixels
        match image::image_dimensions(&scr_path) {
            Ok((w, h)) => {
                if w < 640 || h < 480 {
                    return Err(t("plugin_dev_publish_screenshot_invalid_size")
                        .replace("{}", scr_rel)
                        .replace("{w}", &w.to_string())
                        .replace("{h}", &h.to_string()));
                }
            }
            Err(e) => {
                return Err(t("plugin_dev_publish_screenshot_invalid_format")
                    .replace("{}", scr_rel)
                    .replace("{:?}", &format!("{:?}", e)));
            }
        }
    }

    Ok(())
}

pub fn fetch_or_clone_registry(temp_dir: &std::path::Path) -> anyhow::Result<git2::Repository> {
    if temp_dir.join(".git").exists() {
        if let Ok(repo) = git2::Repository::open(temp_dir) {
            let fetched = {
                if let Ok(mut remote) = repo.find_remote("origin") {
                    let mut fetch_options = git2::FetchOptions::new();
                    remote
                        .fetch(
                            &["+refs/heads/plugin-registry:refs/remotes/origin/plugin-registry"],
                            Some(&mut fetch_options),
                            None,
                        )
                        .is_ok()
                } else {
                    false
                }
            };
            let mut reset_ok = false;
            let mut commit_oid = None;
            if fetched {
                if let Ok(fetch_head) = repo.find_reference("refs/remotes/origin/plugin-registry") {
                    if let Ok(commit) = fetch_head.peel_to_commit() {
                        commit_oid = Some(commit.id());
                    }
                }
            }
            if let Some(oid) = commit_oid {
                if let Ok(commit) = repo.find_commit(oid) {
                    let mut checkout_builder = git2::build::CheckoutBuilder::new();
                    checkout_builder.force();
                    let _ = repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder));
                    let _ = repo.set_head("refs/heads/plugin-registry");
                    if repo
                        .reset(commit.as_object(), git2::ResetType::Hard, None)
                        .is_ok()
                    {
                        reset_ok = true;
                    }
                }
            }
            if reset_ok {
                return Ok(repo);
            }
        }
        // If anything fails, clean up and clone
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    std::fs::create_dir_all(temp_dir)?;

    let url = if let Some(local_path) = find_pairee_repo() {
        log::debug!(
            "plugin-registry: Using local repository clone for registry: {:?}",
            local_path
        );
        local_path.to_string_lossy().into_owned()
    } else {
        log::debug!(
            "plugin-registry: Local repo not found. Cloning registry from remote GitHub..."
        );
        "https://github.com/FittyAr/Pairee.git".to_string()
    };

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch("plugin-registry");
    let repo = builder.clone(&url, temp_dir)?;
    Ok(repo)
}

pub fn package_to_registry(plugin_dir: &std::path::Path) -> anyhow::Result<String> {
    // 1. Validate the plugin
    if let Err(err_msg) = validate_for_publish(plugin_dir) {
        anyhow::bail!(t("plugin_dev_validation_failed").replace("{}", &err_msg));
    }

    let manifest_path = plugin_dir.join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)?;
    let mut manifest = crate::plugin::loader::PluginManifest::parse(&content)?;
    let mut manifest_table: toml::Table = toml::from_str(&content)?;

    // Check for LICENSE file (case-insensitive)
    let mut license_file = None;
    if let Ok(entries) = std::fs::read_dir(plugin_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let name_lower = entry.file_name().to_string_lossy().to_lowercase();
                if name_lower == "license"
                    || name_lower == "license.txt"
                    || name_lower == "license.md"
                {
                    license_file = Some(entry.path());
                    break;
                }
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
    for (rel_path, src_file_path) in crate::plugin::loader::get_plugin_files(plugin_dir) {
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
