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

fn collect_files_rec(dir: &Path, base_dir: &Path, files: &mut Vec<(String, std::path::PathBuf)>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    files.push((rel.to_string_lossy().to_string(), path));
                }
            } else if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with('.')
                {
                    continue;
                }
                collect_files_rec(&path, base_dir, files);
            }
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
    // 1. Read manifest.toml
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("manifest.toml not found for plugin {}", name);
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest = PluginManifest::parse(&manifest_content)?;

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

    // Evaluate Lua and run setup in a nested block to drop all non-Send mlua
    // types before await. Both run under the CPU watchdog.
    let table_key = crate::plugin::limits::guarded(&lua, || {
        let plugin_table: mlua::Table = lua.load(&main_content).eval()?;

        // 5. Run setup() if defined
        if plugin_table.contains_key("setup")? {
            let setup_fn: mlua::Function = plugin_table.get("setup")?;
            let opts = lua.create_table()?;
            setup_fn.call::<()>((plugin_table.clone(), opts))?;
        }

        lua.create_registry_value(plugin_table)
    })?;

    // 6. Register capabilities (Previewers, Commands, Hooks) and spawn task
    crate::plugin::registry::register_plugin(manifest.clone(), table_key, lua, path.to_path_buf())
        .await?;

    tracing::info!(
        "Successfully loaded plugin: {} v{} (License: {:?}, Languages: {:?})",
        manifest.name,
        manifest.version,
        manifest.license,
        manifest.languages
    );
    Ok(())
}

/// Parses a version for the `min_pairee` check: full semver (pre-release
/// and build metadata included), or `X` / `X.Y` padded with zeros.
fn parse_version(v: &str) -> anyhow::Result<semver::Version> {
    let v = v.trim();
    let padded = match v.split(['-', '+']).next().unwrap_or(v).matches('.').count() {
        0 => format!("{v}.0.0"),
        1 => format!("{v}.0"),
        _ => v.to_string(),
    };
    semver::Version::parse(&padded).map_err(|e| anyhow::anyhow!("invalid version {v:?}: {e}"))
}

/// Fails when `current` is older than `min_version` or either cannot be
/// parsed (an unreadable requirement is not silently accepted).
fn check_version_compatibility(min_version: &str, current: &str) -> anyhow::Result<()> {
    let min = parse_version(min_version)?;
    let cur = parse_version(current)?;
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
