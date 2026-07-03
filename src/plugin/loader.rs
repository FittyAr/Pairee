use crate::plugin::manager::PluginRequest;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Sync vs async execution mode for a plugin callback.
///
/// M3 introduces the distinction: a callback declared as `Sync`
/// runs on the main thread and has access to `cx`/`rt`/`th`/`km`
/// live state; an `Async` callback runs in an isolated VM and can
/// only read snapshots. `Entry` is the default for command-style
/// entry points; `Peek` is the default for previewers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// `peek(job)` runs on the main thread; full live context.
    Peek,
    /// `entry(args)` runs on the main thread; full live context.
    Entry,
    /// The callback runs in an isolated VM (no live context).
    Async,
    /// The callback runs on the main thread and may block (e.g.
    /// waiting on `pairee.input`); the re-entry guard is disabled.
    Blocking,
}

impl Default for SyncMode {
    fn default() -> Self {
        SyncMode::Entry
    }
}

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
    pub keybindings: Option<HashMap<String, String>>,
    pub settings_schema: Option<HashMap<String, toml::Value>>,
    /// Default sync mode for callbacks (overridable per-callback
    /// via `--- @sync peek` / `--- @sync entry` annotations in
    /// `main.lua`).
    #[serde(default)]
    pub sync_mode: SyncMode,
    /// Highest Pairee version for which the plugin was tested. Set
    /// via the `--- @since 0.7.0` annotation in `main.lua`; stored
    /// in the manifest after parsing.
    pub since: Option<String>,
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
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                collect_files_rec(&path, base_dir, files);
            }
        }
    }
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
    let mut manifest = PluginManifest::parse(&manifest_content)?;

    // 4b. M3: parse `--- @sync ...` and `--- @since ...` annotations
    //      from the top of `main.lua` and fold them into the
    //      manifest. The parser is line-based and intentionally
    //      tiny: we look for the leading `---` marker, then
    //      `@sync <mode>` or `@since <version>`.
    let main_path = path.join("main.lua");
    if main_path.exists() {
        let main_content = std::fs::read_to_string(&main_path)?;
        if let Some(ann) = parse_annotations(&main_content) {
            if manifest.sync_mode == SyncMode::default() {
                if let Some(mode) = ann.sync {
                    manifest.sync_mode = mode;
                }
            }
            if let Some(since) = ann.since {
                manifest.since = Some(since);
            }
        }
    }

    // 2. Validate version compatibility
    if let Some(ref min_version) = manifest.min_pairee {
        if let Err(e) = check_version_compatibility(min_version) {
            anyhow::bail!("Version check failed for plugin {}: {}", name, e);
        }
    }

    // 3. Create sandboxed Lua instance
    let lua = crate::plugin::sandbox::create_sandboxed_lua(path, trusted, tx.clone())?;

    // 3a. M3: seed `cx` with a snapshot of the live state. The
    //     slim `cx` exposes `active.current.cwd` and
    //     `active.selected`; the full sync-context tree lands in
    //     M4.
    {
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let stub_state = crate::app::state::AppState::new(workspace.clone(), workspace);
        let _ = crate::plugin::runtime::bindings::cx::build_cx_table(&lua, &stub_state);
    }

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

        // 5b. M3: attach the plugin table as `pairee.state` so
        //     callbacks can mutate it freely across calls. We
        //     create a NEW empty table for `pairee.state` (the
        //     plugin's main return value is the public API
        //     surface; `pairee.state` is the scratch space).
        if let Some(rt) = lua.app_data_ref::<crate::plugin::runtime::runtime::Runtime>() {
            let state_table = lua.create_table()?;
            crate::plugin::runtime::bindings::state::attach(
                &lua,
                &rt,
                name,
                &state_table,
            )?;
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

/// Result of parsing the `--- @sync ...` and `--- @since ...`
/// annotations at the top of `main.lua`. Only the *first*
/// annotation of each kind is honoured; later ones are
/// ignored (so a plugin author who puts `@sync peek` then
/// `@sync entry` will get the first one).
#[derive(Debug, Default, PartialEq, Eq)]
struct Annotations {
    sync: Option<SyncMode>,
    since: Option<String>,
}

fn parse_annotations(main_content: &str) -> Option<Annotations> {
    let mut ann = Annotations::default();
    for line in main_content.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("---") {
            // Blank line ends the annotation header block (a
            // common convention in Lua's `---` header comments).
            if trimmed.is_empty() {
                continue;
            }
            // First non-`---` line ends the scan.
            if ann.sync.is_some() || ann.since.is_some() {
                break;
            }
            continue;
        }
        let body = trimmed[3..].trim();
        if let Some(rest) = body.strip_prefix("@sync") {
            let mode_str = rest.trim();
            ann.sync = match mode_str {
                "peek" => Some(SyncMode::Peek),
                "entry" => Some(SyncMode::Entry),
                "async" => Some(SyncMode::Async),
                "blocking" => Some(SyncMode::Blocking),
                _ => ann.sync,
            };
        } else if let Some(rest) = body.strip_prefix("@since") {
            ann.since = Some(rest.trim().to_string());
        }
    }
    if ann.sync.is_some() || ann.since.is_some() {
        Some(ann)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn test_parse_annotations_sync_peek() {
        let main = r#"
            --- @sync peek
            --- @since 0.7.0
            --- A previewer that needs live state.
            return { peek = function() end }
        "#;
        let ann = parse_annotations(main).expect("annotations parsed");
        assert_eq!(ann.sync, Some(SyncMode::Peek));
        assert_eq!(ann.since.as_deref(), Some("0.7.0"));
    }

    #[test]
    fn test_parse_annotations_async() {
        let main = "--- @sync async\nlocal _ = 1\n";
        let ann = parse_annotations(main).expect("annotations parsed");
        assert_eq!(ann.sync, Some(SyncMode::Async));
        assert!(ann.since.is_none());
    }

    #[test]
    fn test_parse_annotations_no_annotations() {
        let main = "local M = {}\nfunction M.peek() end\nreturn M\n";
        assert!(parse_annotations(main).is_none());
    }
}
