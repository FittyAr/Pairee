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
    pub keybindings: Option<HashMap<String, String>>,
    pub settings_schema: Option<HashMap<String, toml::Value>>,
}

pub async fn load_plugin(
    name: &str,
    path: &Path,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<()> {
    // 1. Read manifest.toml
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("manifest.toml not found for plugin {}", name);
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = toml::from_str(&manifest_content)?;

    // 2. Validate version compatibility
    if let Some(ref min_version) = manifest.min_pairee {
        if let Err(e) = check_version_compatibility(min_version) {
            anyhow::bail!("Version check failed for plugin {}: {}", name, e);
        }
    }

    // 3. Create sandboxed Lua instance
    let lua = crate::plugin::sandbox::create_sandboxed_lua(path, trusted, tx.clone())?;

    // 4. Load main.lua
    let main_path = path.join("main.lua");
    if !main_path.exists() {
        anyhow::bail!("main.lua not found for plugin {}", name);
    }
    let main_content = std::fs::read_to_string(&main_path)?;

    // Evaluate Lua and run setup in a nested block to drop all non-Send mlua types before await
    let table_key = {
        let plugin_table: mlua::Table = lua.load(&main_content).eval()?;

        // 5. Run setup() if defined
        if plugin_table.contains_key("setup")? {
            let setup_fn: mlua::Function = plugin_table.get("setup")?;
            let opts = lua.create_table()?;
            setup_fn.call::<_, ()>((plugin_table.clone(), opts))?;
        }

        lua.create_registry_value(plugin_table)?
    };

    // 6. Register capabilities (Previewers, Commands, Hooks) and spawn task
    crate::plugin::registry::register_plugin(manifest.clone(), table_key, lua, path.to_path_buf())
        .await?;

    log::info!(
        "Successfully loaded plugin: {} v{} (License: {:?}, Languages: {:?})",
        manifest.name,
        manifest.version,
        manifest.license,
        manifest.languages
    );
    Ok(())
}

fn check_version_compatibility(min_version: &str) -> anyhow::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Parse helper
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            let maj = parts[0].parse().ok()?;
            let min = parts[1].parse().ok()?;
            let pat = parts[2].parse().ok()?;
            Some((maj, min, pat))
        } else {
            None
        }
    };

    if let (Some(cur), Some(min)) = (parse(current_version), parse(min_version)) {
        if cur >= min {
            Ok(())
        } else {
            anyhow::bail!(
                "Pairee version {} is less than required minimum {}",
                current_version,
                min_version
            );
        }
    } else {
        // Fallback to allow if parsing fails
        Ok(())
    }
}
