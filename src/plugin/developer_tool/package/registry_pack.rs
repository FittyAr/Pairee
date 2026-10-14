use super::super::{progress_progress, progress_status};
use super::license::AskLicense;
use super::registry_repo::fetch_or_clone_registry;
use super::validate::validate_for_publish_with_progress;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use crate::plugin::loader::PluginManifest;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

/// CLI packaging: `ask_license` asks for a license name when needed.
pub fn package_to_registry(
    plugin_dir: &Path,
    ask_license: AskLicense<'_>,
) -> anyhow::Result<String> {
    package_impl(plugin_dir, None, ask_license)
}

/// TUI packaging with progress updates (never prompts on the terminal).
pub fn package_to_registry_with_progress(
    plugin_dir: &Path,
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<String> {
    package_impl(plugin_dir, progress, None)
}

fn package_impl(
    plugin_dir: &Path,
    progress: Option<UnboundedSender<DevProgress>>,
    ask_license: AskLicense<'_>,
) -> anyhow::Result<String> {
    // 1. Validate the plugin
    if let Err(err_msg) = validate_for_publish_with_progress(plugin_dir, progress.clone()) {
        anyhow::bail!(t("plugin_dev_validation_failed").replace("{}", &err_msg));
    }

    let manifest_path = plugin_dir.join("manifest.toml");
    progress_status(&progress, t("plugin_dev_progress_reading_manifest"));
    let content = std::fs::read_to_string(&manifest_path)?;
    let mut manifest = PluginManifest::parse(&content)?;
    // Name and author become folder names: check them before anything is
    // fetched, written or deleted.
    let temp_dir = crate::plugin::developer_tool::temp_registry_dir();
    let dest_plugin_dir = crate::plugin::developer_tool::registry_plugin_dir(&temp_dir, &manifest)?;
    // The author's folder is never modified: the license and the checksums
    // go into the registry copy only.
    let license = super::license::resolve_license(plugin_dir, &manifest, ask_license);
    manifest.license = Some(license.name.clone());

    // 2. Clone or update the registry repo in temporary directory
    progress_status(&progress, t("plugin_dev_progress_fetching_registry"));
    let _repo = fetch_or_clone_registry(&temp_dir)?;

    // 3. Copy plugin files to the cloned repo
    if dest_plugin_dir.exists() {
        let _ = std::fs::remove_dir_all(&dest_plugin_dir);
    }
    std::fs::create_dir_all(&dest_plugin_dir)?;
    let mut files_hash = copy_plugin_files(plugin_dir, &dest_plugin_dir, &progress)?;
    if let Some(text) = &license.generated_file {
        let file = dest_plugin_dir.join("LICENSE");
        std::fs::write(&file, text)?;
        files_hash.insert(
            "LICENSE".to_string(),
            crate::update::downloader::compute_sha256(&file)?,
        );
    }
    write_checksums(&content, &license.name, &dest_plugin_dir, &files_hash)?;

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
    // The manifest is published with its `[files]` table (it cannot list
    // its own hash) and `sha256.sum` is generated here.
    let files: Vec<_> = files
        .into_iter()
        .filter(|(rel, _)| rel != "manifest.toml" && rel != "sha256.sum")
        .collect();
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

/// Writes `sha256.sum` and the published manifest: the author's manifest
/// text with `license` set and its `[files]` table **replaced** by the
/// checksums (a second `[files]` table would not parse).
fn write_checksums(
    source_manifest: &str,
    license: &str,
    dest_plugin_dir: &Path,
    files_hash: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let mut sha_content = String::new();
    let sorted: std::collections::BTreeMap<_, _> = files_hash.iter().collect();
    for (f, h) in &sorted {
        sha_content.push_str(&format!("{}  {}\n", h, f));
    }
    std::fs::write(dest_plugin_dir.join("sha256.sum"), sha_content)?;
    std::fs::write(
        dest_plugin_dir.join("manifest.toml"),
        published_manifest(source_manifest, license, files_hash)?,
    )?;
    Ok(())
}

/// `source` with `license` set (inside `[plugin]` when it has one) and
/// `[files]` replaced by `files_hash`.
fn published_manifest(
    source: &str,
    license: &str,
    files_hash: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut table: toml::Table = toml::from_str(source)?;
    let license = toml::Value::String(license.to_string());
    match table.get_mut("plugin") {
        Some(toml::Value::Table(plugin)) => {
            plugin.insert("license".to_string(), license);
        }
        _ => {
            table.insert("license".to_string(), license);
        }
    }
    let files: toml::Table = files_hash
        .iter()
        .map(|(path, hash)| (path.clone(), toml::Value::String(hash.clone())))
        .collect();
    table.insert("files".to_string(), toml::Value::Table(files));
    Ok(toml::to_string_pretty(&table)?)
}

/// Adds or replaces the plugin entry of `registry/index.toml`.
fn update_index(index_path: &Path, manifest: &PluginManifest) -> anyhow::Result<()> {
    use crate::plugin::updater::{RegistryIndex, RegistryPlugin};
    let empty = || RegistryIndex {
        plugins: HashMap::new(),
    };
    let mut index_data = if index_path.exists() {
        // An unreadable index is an error: an empty one would drop every
        // other plugin from the registry in the next commit.
        let content = std::fs::read_to_string(index_path)?;
        toml::from_str::<RegistryIndex>(&content)
            .map_err(|e| anyhow::anyhow!("{} is not a valid index: {e}", index_path.display()))?
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
        // The manifest does not declare the hook events a plugin handles;
        // its `[keybindings]` actions are not hooks.
        hooks: None,
        min_pairee: manifest.min_pairee.clone(),
    };
    index_data.plugins.insert(manifest.name.clone(), reg_plugin);

    std::fs::write(index_path, toml::to_string_pretty(&index_data)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keybinding_actions_are_not_listed_as_hooks() {
        let manifest = PluginManifest::parse(
            "name = \"demo\"
version = \"1.0.0\"
[keybindings]
\"ctrl+h\" = \"hello\"
",
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("registry").join("index.toml");
        update_index(&index_path, &manifest).unwrap();
        let index: crate::plugin::updater::RegistryIndex =
            toml::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(index.plugins["demo"].hooks, None);
    }

    #[test]
    fn a_broken_index_is_an_error_not_an_empty_registry() {
        let manifest = PluginManifest::parse("name = \"demo\"\nversion = \"1.0.0\"\n").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("index.toml");
        std::fs::write(&index_path, "[plugins.other\nbroken").unwrap();
        assert!(update_index(&index_path, &manifest).is_err());
        assert_eq!(
            std::fs::read_to_string(&index_path).unwrap(),
            "[plugins.other\nbroken"
        );
    }

    #[test]
    fn packaged_files_exclude_the_manifest_and_hidden_files() {
        let src = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("lang")).unwrap();
        for f in ["manifest.toml", "main.lua", "lang/en.toml", ".env"] {
            std::fs::write(src.path().join(f), "x = 1\n").unwrap();
        }
        let hashes = copy_plugin_files(src.path(), dest.path(), &None).unwrap();
        let mut keys: Vec<_> = hashes.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["lang/en.toml", "main.lua"]);
        write_checksums(
            "name = \"x\"\nversion = \"1\"\n",
            "MIT",
            dest.path(),
            &hashes,
        )
        .unwrap();
        let written = std::fs::read_to_string(dest.path().join("manifest.toml")).unwrap();
        let parsed: crate::plugin::updater::types::RegistryPluginManifestWrapper =
            toml::from_str(&written).unwrap();
        assert_eq!(parsed.files.unwrap().len(), 2);
    }

    #[test]
    fn an_existing_files_table_is_replaced_and_the_license_set() {
        let source = "name = \"demo\"\nversion = \"1\"\n\n[files]\n\"old.lua\" = \"00\"\n";
        let hashes: HashMap<String, String> = [("main.lua".to_string(), "ab".to_string())].into();
        let text = published_manifest(source, "MIT", &hashes).unwrap();
        let parsed: crate::plugin::updater::types::RegistryPluginManifestWrapper =
            toml::from_str(&text).unwrap();
        assert_eq!(
            parsed.files.unwrap().keys().collect::<Vec<_>>(),
            ["main.lua"]
        );
        assert_eq!(
            PluginManifest::parse(&text).unwrap().license.as_deref(),
            Some("MIT")
        );
        let nested = "[plugin]\nname = \"demo\"\nversion = \"1\"\n";
        let text = published_manifest(nested, "Apache-2.0", &hashes).unwrap();
        assert_eq!(
            PluginManifest::parse(&text).unwrap().license.as_deref(),
            Some("Apache-2.0")
        );
    }

    #[test]
    fn registry_folders_need_plain_names() {
        let temp = Path::new("reg");
        for (name, author) in [
            ("../../x", "me"),
            ("ok", "../x"),
            ("ok", "Jane Doe"),
            ("C:x", "me"),
        ] {
            let manifest = PluginManifest::parse(&format!(
                "name = {name:?}\nversion = \"1\"\nauthor = {author:?}\n"
            ))
            .unwrap();
            assert!(
                crate::plugin::developer_tool::registry_plugin_dir(temp, &manifest).is_err(),
                "{name} {author}"
            );
        }
        let manifest =
            PluginManifest::parse("name = \"demo\"\nversion = \"1\"\nauthor = \"Bob\"\n").unwrap();
        assert_eq!(
            crate::plugin::developer_tool::registry_plugin_dir(temp, &manifest).unwrap(),
            temp.join("registry").join("plugins/b/Bob/demo")
        );
    }
}
