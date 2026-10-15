use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::manager::PluginRequest;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub min_pairee: Option<String>,
    pub requires_trust: Option<bool>,
    pub default_language: Option<String>,
    pub languages: Option<Vec<String>>,
    pub icon: Option<String>,
    pub screenshots: Option<Vec<String>>,
    /// Pre-v2 `"key" = "action"` table (see [`super::commands_decl`]).
    pub keybindings: Option<HashMap<String, String>>,
    /// `[[commands]]`: commands with the keys they suggest per preset.
    #[serde(default)]
    pub commands: Vec<super::commands_decl::CommandDecl>,
    pub settings_schema: Option<HashMap<String, toml::Value>>,
    #[serde(default)]
    pub permissions: PluginPermissions,
}

/// `[permissions]` table of `manifest.toml`.
#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct PluginPermissions {
    /// Programs the plugin may spawn by name when Secure Mode is on.
    #[serde(default)]
    pub commands: Vec<String>,
}

impl PluginManifest {
    pub fn parse(content: &str) -> anyhow::Result<Self> {
        let mut table: toml::Table = toml::from_str(content)?;
        if let Some(toml::Value::Table(plugin_table)) = table.remove("plugin") {
            for (k, v) in plugin_table {
                table.insert(k, v);
            }
        }
        let manifest: Self = toml::Value::Table(table).try_into()?;
        Ok(manifest)
    }

    /// The `default_language` code (`en` when missing or not a plain
    /// `[A-Za-z0-9_-]` code, which could name a file outside `lang/`).
    pub fn default_language(&self) -> &str {
        self.default_language
            .as_deref()
            .filter(|code| crate::plugin::updater::is_identifier(code))
            .unwrap_or("en")
    }

    /// The commands this plugin declares, in keymap form.
    pub fn keymap_commands(&self) -> Vec<crate::keybindings::plugin_commands::PluginCommand> {
        super::commands_decl::plugin_commands(&self.name, &self.commands, self.keybindings.as_ref())
    }

    /// Read and parse `<plugin_dir>/manifest.toml`, if present and valid.
    pub fn read_from_dir(plugin_dir: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(plugin_dir.join("manifest.toml")).ok()?;
        Self::parse(&content).ok()
    }
}

pub fn get_plugin_files(plugin_dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut files = Vec::new();
    collect_files_rec(plugin_dir, plugin_dir, &mut files);
    files
}

/// Files under `dir` as (`/`-separated path relative to `base_dir`, path).
/// Hidden entries (`.git`, `.env`, ...) and symbolic links are skipped: a
/// published plugin must not carry secrets or files from outside its
/// folder, and a link loop cannot recurse forever.
fn collect_files_rec(dir: &Path, base_dir: &Path, files: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if hidden || kind.is_symlink() {
            continue;
        }
        if kind.is_file() {
            if let Ok(rel) = path.strip_prefix(base_dir) {
                let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
                files.push((parts.join("/"), path));
            }
        } else if kind.is_dir() {
            collect_files_rec(&path, base_dir, files);
        }
    }
}

/// Loads the plugin in `path` with the host's `options` and registers it.
pub async fn load_plugin(
    path: &Path,
    options: &PluginLoadOptions,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<()> {
    let name = &options.name;
    let generation = crate::plugin::registry::generation();
    // 1. Read manifest.toml
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("manifest.toml not found for plugin {}", name);
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest = PluginManifest::parse(&manifest_content)?;
    // Trust, settings and the data folder follow the folder name; the
    // loaded-plugin registry, keymap commands and events use the manifest
    // name. They must agree, or a plugin could take another's identity
    // (and replace its worker) by naming it in its manifest.
    if manifest.name != *name {
        anyhow::bail!(
            "plugin folder {name}.pairee declares the name {:?}; they must match",
            manifest.name
        );
    }

    // 2. Validate version compatibility
    if let Some(ref min_version) = manifest.min_pairee
        && let Err(e) = check_version_compatibility(min_version, env!("CARGO_PKG_VERSION"))
    {
        anyhow::bail!("Version check failed for plugin {}: {}", name, e);
    }

    // 3. Create sandboxed Lua instance
    let lua = crate::plugin::sandbox::create_sandboxed_lua(path, options, tx.clone())?;

    // 4. Load main.lua
    let main_path = path.join("main.lua");
    if !main_path.exists() {
        anyhow::bail!("main.lua not found for plugin {}", name);
    }
    let main_content = std::fs::read_to_string(&main_path)?;
    let annotations = crate::plugin::annotations::Annotations::parse(&main_content);
    annotations
        .check_since(crate::plugin::runtime::api_version::LUA_API_VERSION)
        .map_err(|e| anyhow::anyhow!("Plugin {name}: {e}"))?;

    // Evaluate main.lua, then run setup() if defined, both under the CPU
    // watchdog. setup runs asynchronously, like every other plugin method,
    // so it may await dialogs or `pairee.sync`.
    // Reading fields of the returned table may run its metamethods, so the
    // lookups stay under the watchdog as well.
    let (plugin_table, setup) = crate::plugin::limits::guarded(&lua, || {
        let table: mlua::Table = lua.load(&main_content).eval()?;
        let setup = table.get::<mlua::Value>("setup")?;
        Ok((table, setup))
    })?;
    if let mlua::Value::Function(setup_fn) = setup {
        let opts = lua.create_table()?;
        let call = setup_fn.call_async::<()>((plugin_table.clone(), opts));
        crate::plugin::limits::guarded_async(&lua, call).await?;
    }
    let methods = crate::plugin::limits::guarded(&lua, || {
        Ok(crate::plugin::registry::PluginMethods::of(&plugin_table))
    })?;
    let table_key = lua.create_registry_value(plugin_table)?;

    // 6. Register capabilities (Previewers, Commands, Hooks) and spawn task
    let loaded = crate::plugin::registry::LoadedPlugin {
        manifest: manifest.clone(),
        path: path.to_path_buf(),
        lua,
        table_key,
        annotations,
        methods,
    };
    crate::plugin::registry::register_plugin(loaded, tx, generation).await?;

    tracing::info!(
        "Successfully loaded plugin: {} v{} (License: {:?}, Languages: {:?})",
        manifest.name,
        manifest.version,
        manifest.license,
        manifest.languages
    );
    Ok(())
}

/// Fails when `current` is older than `min_version` or either cannot be
/// parsed (an unreadable requirement is not silently accepted).
pub(crate) fn check_version_compatibility(min_version: &str, current: &str) -> anyhow::Result<()> {
    let min = crate::update::version::parse(min_version)?;
    let cur = crate::update::version::parse(current)?;
    if cur >= min {
        Ok(())
    } else {
        anyhow::bail!("Pairee version {current} is less than required minimum {min_version}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_language_must_be_a_plain_code() {
        let manifest = |lang: &str| {
            PluginManifest::parse(&format!(
                "name = \"x\"\nversion = \"1\"\ndefault_language = {lang:?}\n"
            ))
            .unwrap()
        };
        assert_eq!(manifest("es").default_language(), "es");
        assert_eq!(manifest("pt-BR").default_language(), "pt-BR");
        for bad in ["../../../home/u/.cargo/credentials", "/etc/x", "C:x", ""] {
            assert_eq!(manifest(bad).default_language(), "en", "{bad}");
        }
    }

    #[test]
    fn version_check_accepts_newer_and_prerelease() {
        assert!(check_version_compatibility("0.7.0", "0.10.0").is_ok());
        assert!(check_version_compatibility("0.10.0", "0.10.0").is_ok());
        assert!(check_version_compatibility("0.10.0-beta.1", "0.10.0").is_ok());
        assert!(check_version_compatibility("0.9", "0.10.0").is_ok());
        assert!(check_version_compatibility("1", "0.10.0").is_err());
        assert!(check_version_compatibility("0.11.0", "0.10.0").is_err());
        assert!(check_version_compatibility("0.10.0", "0.10.0-rc.1").is_err());
    }

    #[test]
    fn version_check_rejects_unparsable_requirements() {
        for bad in ["", "latest", "0.x.1", "v0.7.0", ">=0.7"] {
            assert!(check_version_compatibility(bad, "0.10.0").is_err(), "{bad}");
        }
    }

    #[test]
    fn test_parse_manifest_flat() {
        let content = r#"
            name = "test_plugin"
            version = "0.1.0"
            description = "A flat manifest"
        "#;
        let manifest = PluginManifest::parse(content).unwrap();
        assert_eq!(manifest.name, "test_plugin");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.description.as_deref(), Some("A flat manifest"));
    }

    #[test]
    fn test_parse_manifest_nested() {
        let content = r#"
            [plugin]
            name = "test_plugin"
            version = "0.1.0"
            description = "A nested manifest"

            [keybindings]
            "ctrl-p" = "my_custom_action"
        "#;
        let manifest = PluginManifest::parse(content).unwrap();
        assert_eq!(manifest.name, "test_plugin");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.description.as_deref(), Some("A nested manifest"));
        let kbs = manifest.keybindings.unwrap();
        assert_eq!(
            kbs.get("ctrl-p").map(|s| s.as_str()),
            Some("my_custom_action")
        );
        assert!(manifest.permissions.commands.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn setup_may_await_and_since_is_enforced() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("async-setup.pairee");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.toml"),
            "name = \"async-setup\"
version = \"1.0.0\"
",
        )
        .unwrap();
        std::fs::write(
            dir.join("main.lua"),
            "local M = {}
function M:setup() pairee.utils.sleep(0) end
return M
",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(4);
        let options = PluginLoadOptions::new("async-setup", false, false);
        load_plugin(&dir, &options, tx.clone()).await.unwrap();
        crate::plugin::registry::unregister_plugin("async-setup").await;

        std::fs::write(
            dir.join("main.lua"),
            "--- @since 99.0
return {}
",
        )
        .unwrap();
        let err = load_plugin(&dir, &options, tx).await.unwrap_err();
        assert!(err.to_string().contains("@since 99.0"), "{err}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_plugin_cannot_claim_another_plugins_name() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("evil.pairee");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.toml"),
            "name = \"git-status\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("main.lua"), "return {}").unwrap();
        let (tx, _rx) = mpsc::channel(4);
        let options = PluginLoadOptions::new("evil", false, false);
        let err = load_plugin(&dir, &options, tx).await.unwrap_err();
        assert!(err.to_string().contains("must match"), "{err}");
        assert!(
            crate::plugin::registry::plugin_info("git-status")
                .await
                .is_none()
        );
    }

    #[test]
    fn test_parse_manifest_permissions() {
        let content = r#"
            [plugin]
            name = "p"
            version = "0.1.0"

            [permissions]
            commands = ["git", "rg"]
        "#;
        let manifest = PluginManifest::parse(content).unwrap();
        assert_eq!(manifest.permissions.commands, vec!["git", "rg"]);
    }
}
