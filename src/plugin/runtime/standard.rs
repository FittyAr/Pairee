use crate::plugin::manager::PluginRequest;
use std::path::Path;
use tokio::sync::mpsc;

pub fn bind_runtime(
    lua: &mlua::Lua,
    plugin_dir: &Path,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    let globals = lua.globals();

    // Create central pairee table
    let pairee = lua.create_table()?;

    // 1. Bind global secure_mode parameter
    let mut secure_mode_active = false;
    // Let's get the active config to see if secure_mode is true
    if let Ok(config) = crate::config::AppConfig::load_or_create() {
        secure_mode_active = config.settings.secure_mode;
    }
    pairee.set("_secure_mode", secure_mode_active)?;

    // 2. Bind submodules
    pairee.set("app", super::bindings::app::bind(lua, tx.clone())?)?;
    pairee.set("emit", super::bindings::emit::bind(lua, tx.clone())?)?;
    pairee.set(
        "file_cache",
        super::bindings::file_cache::bind(lua, tx.clone())?,
    )?;
    // Dialogs write their entries directly on the central `pairee`
    // table so that the top-level `pairee.confirm` and `pairee.input`
    // are discoverable alongside the existing `pairee.app.*` stubs.
    super::bindings::dialogs::bind(lua, &pairee, tx.clone())?;
    pairee.set("fs", super::bindings::fs::bind(lua, trusted, tx.clone())?)?;
    pairee.set("ui", super::bindings::ui::bind(lua)?)?;
    pairee.set("ps", super::bindings::ps::bind(lua, tx.clone())?)?;
    pairee.set("log", super::bindings::log::bind(lua)?)?;
    pairee.set(
        "notify",
        super::bindings::notify_ext::bind(lua, tx.clone())?,
    )?;
    pairee.set("sync", super::bindings::sync::bind(lua, tx.clone())?)?;
    // M3: `pairee.async_fn(fn)` is a no-op shim today (M4 will
    // introduce the sync/async VM split). It exists so plugins
    // can write `pairee.async_fn(do_thing)` ahead of M4.
    pairee.set("async_fn", super::bindings::sync::bind_async(lua, tx.clone())?)?;
    // utils_ext composes on top of utils_basic, so the M0 set
    // (target_os, target_family, time, hash) remains available, and the
    // M1 helpers (quote, percent_encode, percent_decode, json_encode,
    // json_decode, sleep) are added on top.
    pairee.set("utils", super::bindings::utils_ext::bind(lua)?)?;
    pairee.set("which", super::bindings::which::bind(lua, tx.clone())?)?;

    // 3. Bind settings
    bind_settings(lua, &pairee, plugin_dir)?;

    // 3.5 M2 typed userdata: `pairee.Url`, `pairee.Path`, `pairee.Cha`,
    //     `pairee.File`, `pairee.Error`, and the `Err(s, ...)` helper.
    crate::plugin::runtime::types::register(lua, &pairee)?;
    // M2 image binding: `pairee.image.{show, precache, info}`.
    pairee.set("image", super::bindings::image::bind(lua, tx.clone())?)?;

    // M3 process binding: `pairee.Command(name)`, `pairee.fs.access()`.
    super::bindings::process::command::register(lua, &pairee)?;
    let fs_table: mlua::Table = pairee.get("fs")?;
    super::bindings::process::access::register(lua, &fs_table)?;

    // M3: seed a fresh `Runtime` on the Lua app data so the
    //      `runtime_scope!` macro (and the `Runtime::is_blocking`
    //      re-entry guard) have a place to live.
    if lua.app_data_ref::<super::runtime::Runtime>().is_none() {
        lua.set_app_data(super::runtime::Runtime::new());
    }

    // 4. Top-level aliases for the M1 utility surface so plugins can
    //    also write `pairee.quote(...)` / `pairee.sleep(...)` etc. —
    //    matches the public shape described in the plugin roadmap
    //    (docs/technical/plugin-roadmap.md §5.E9).
    register_top_level_aliases(lua, &pairee, &tx)?;

    // 4. Bind i18n translation helper: pairee.t
    bind_translations(lua, &pairee, plugin_dir)?;

    globals.set("pairee", pairee)?;
    Ok(())
}

/// Registers top-level aliases for entries that already exist on
/// `pairee.utils` (and on `pairee.clipboard` once the M1 binding is
/// installed). Mirrors the public shape described in the plugin
/// roadmap §5.E9 — `pairee.quote`, `pairee.sleep`, `pairee.time`,
/// `pairee.hash`, `pairee.target_os`, `pairee.target_family`,
/// `pairee.percent_*`, `pairee.json_*`, plus the Unix identity
/// helpers. Aliases for keys not yet present in `pairee.utils` are
/// silently skipped so this function is forward-compatible with
/// later M1/M2 additions.
fn register_top_level_aliases(
    lua: &mlua::Lua,
    pairee: &mlua::Table<'_>,
    tx: &mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    let utils: mlua::Table = pairee.get("utils")?;
    let alias_keys: &[&str] = &[
        "quote",
        "percent_encode",
        "percent_decode",
        "json_encode",
        "json_decode",
        "sleep",
        "time",
        "hash",
        "target_os",
        "target_family",
        "uid",
        "gid",
        "user_name",
        "group_name",
        "host_name",
    ];
    for key in alias_keys {
        if let Ok(v) = utils.get::<_, mlua::Value>(*key) {
            if !matches!(v, mlua::Value::Nil) {
                pairee.set(*key, v)?;
            }
        }
    }
    // `pairee.clipboard(...)` — bind a thin alias over the dedicated
    // binding table so both `pairee.clipboard.text = ...` and
    // `pairee.clipboard(...)` work once the M1 binding is installed.
    if let Ok(cb) = super::bindings::clipboard::bind(lua, tx.clone()) {
        if let Ok(cb_fn) = cb.get::<_, mlua::Function>("clipboard") {
            pairee.set("clipboard", cb_fn)?;
        }
    }
    Ok(())
}


fn bind_settings(lua: &mlua::Lua, pairee: &mlua::Table<'_>, plugin_dir: &Path) -> mlua::Result<()> {
    let settings_table = lua.create_table()?;

    // Read manifest schema
    let manifest_path = plugin_dir.join("manifest.toml");
    let mut plugin_name = String::new();
    let mut default_settings = std::collections::HashMap::new();

    if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = crate::plugin::loader::PluginManifest::parse(&content) {
                plugin_name = manifest.name.clone();
                if let Some(schema) = manifest.settings_schema {
                    for (k, v) in schema {
                        // Extract default value
                        if let Some(tbl) = v.as_table() {
                            if let Some(default_val) = tbl.get("default") {
                                default_settings.insert(k, default_val.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Load active settings from config
    let mut active_settings = std::collections::HashMap::new();
    if let Ok(config) = crate::config::AppConfig::load_or_create() {
        if let Some(user_settings) = config.settings.plugin_settings.get(&plugin_name) {
            for (k, v) in user_settings {
                active_settings.insert(k.clone(), v.clone());
            }
        }
    }

    // Merge default and user active settings, populate Lua settings table
    for (k, def) in default_settings {
        let val_str = active_settings.get(&k);
        let val: mlua::Value = match val_str {
            Some(s) => {
                // Parse string back to appropriate type
                if let Ok(b) = s.parse::<bool>() {
                    mlua::Value::Boolean(b)
                } else if let Ok(i) = s.parse::<i64>() {
                    mlua::Value::Integer(i)
                } else {
                    mlua::Value::String(lua.create_string(s)?)
                }
            }
            None => {
                // Use default
                match def {
                    toml::Value::Boolean(b) => mlua::Value::Boolean(b),
                    toml::Value::Integer(i) => mlua::Value::Integer(i),
                    toml::Value::String(s) => mlua::Value::String(lua.create_string(&s)?),
                    _ => mlua::Value::Nil,
                }
            }
        };
        settings_table.set(k, val)?;
    }

    pairee.set("settings", settings_table)?;
    Ok(())
}

fn bind_translations(
    lua: &mlua::Lua,
    pairee: &mlua::Table<'_>,
    plugin_dir: &Path,
) -> mlua::Result<()> {
    let mut default_lang = "en".to_string();
    let manifest_path = plugin_dir.join("manifest.toml");

    if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = crate::plugin::loader::PluginManifest::parse(&content) {
                if let Some(ref dl) = manifest.default_language {
                    default_lang = dl.clone();
                }
            }
        }
    }

    let lang_dir = plugin_dir.join("lang");
    let current_lang = crate::config::localization::get_active_language_code();

    // Load active locale TOML
    let mut active_dict = toml::Table::default();
    let active_path = lang_dir.join(format!("{}.toml", current_lang));
    if active_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&active_path) {
            if let Ok(dict) = toml::from_str::<toml::Table>(&content) {
                active_dict = dict;
            }
        }
    }

    // Load fallback default locale TOML
    let mut fallback_dict = toml::Table::default();
    let fallback_path = lang_dir.join(format!("{}.toml", default_lang));
    if fallback_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&fallback_path) {
            if let Ok(dict) = toml::from_str::<toml::Table>(&content) {
                fallback_dict = dict;
            }
        }
    }

    let t_fn = lua.create_function(
        move |_lua_ctx, (key, vars): (String, Option<mlua::Table>)| {
            let lookup = |dict: &toml::Table, k: &str| -> Option<String> {
                let parts: Vec<&str> = k.split('.').collect();
                let mut current = dict;
                for (i, &part) in parts.iter().enumerate() {
                    if let Some(val) = current.get(part) {
                        if i == parts.len() - 1 {
                            return val.as_str().map(|s| s.to_string());
                        } else if let Some(tbl) = val.as_table() {
                            current = tbl;
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                None
            };

            // Resolve value from active or fallback dictionary
            let mut raw_val = lookup(&active_dict, &key)
                .or_else(|| lookup(&fallback_dict, &key))
                .unwrap_or_else(|| format!("[{}]", key));

            // Perform variable interpolation
            if let Some(tbl) = vars {
                for pair in tbl.pairs::<String, String>().flatten() {
                    let placeholder = format!("{{{}}}", pair.0);
                    raw_val = raw_val.replace(&placeholder, &pair.1);
                }
            }

            Ok(raw_val)
        },
    )?;

    pairee.set("t", t_fn)?;
    Ok(())
}
