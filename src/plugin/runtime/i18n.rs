//! `pairee.t(key, vars)`: the plugin's own translations (`lang/<code>.toml`
//! in its folder), looked up with Pairee's shared translator: the active
//! UI language first, then the plugin's `default_language`, then `[key]`.
//! Nested tables give dotted keys (`[menu] open` → `menu.open`).

use crate::config::localization::translator::{Dictionary, flatten_table, interpolate, lookup};
use crate::plugin::loader::PluginManifest;
use std::path::Path;

/// Reads `<lang_dir>/<code>.toml` as a flat dictionary (empty on error).
fn read_dictionary(lang_dir: &Path, code: &str) -> Dictionary {
    std::fs::read_to_string(lang_dir.join(format!("{code}.toml")))
        .ok()
        .and_then(|content| toml::from_str::<toml::Table>(&content).ok())
        .map(|table| flatten_table(&table))
        .unwrap_or_default()
}

/// Builds the `pairee.t` function for the plugin in `plugin_dir`.
pub fn bind(
    lua: &mlua::Lua,
    plugin_dir: &Path,
    manifest: Option<&PluginManifest>,
) -> mlua::Result<mlua::Function> {
    let default_lang = manifest
        .and_then(|m| m.default_language.clone())
        .unwrap_or_else(|| "en".to_string());
    let lang_dir = plugin_dir.join("lang");
    let active = read_dictionary(
        &lang_dir,
        &crate::config::localization::get_active_language_code(),
    );
    let fallback = read_dictionary(&lang_dir, &default_lang);

    lua.create_function(move |_, (key, vars): (String, Option<mlua::Table>)| {
        let text =
            lookup(&active, &fallback, &key).map_or_else(|| format!("[{key}]"), str::to_owned);
        let Some(vars) = vars else {
            return Ok(text);
        };
        let pairs: Vec<(String, String)> = vars.pairs::<String, String>().flatten().collect();
        Ok(interpolate(
            &text,
            pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_translations_use_fallback_and_vars() {
        let dir = tempfile::tempdir().unwrap();
        let lang = dir.path().join("lang");
        std::fs::create_dir(&lang).unwrap();
        std::fs::write(
            lang.join("en.toml"),
            "hello = \"Hello {name}\"\n[menu]\nopen = \"Open\"\n",
        )
        .unwrap();
        let lua = mlua::Lua::new();
        let t = bind(&lua, dir.path(), None).unwrap();
        lua.globals().set("t", t).unwrap();
        let (a, b, c): (String, String, String) = lua
            .load("return t('hello', { name = 'Ana' }), t('menu.open'), t('nope')")
            .eval()
            .unwrap();
        assert_eq!(a, "Hello Ana");
        assert_eq!(b, "Open");
        assert_eq!(c, "[nope]");
    }
}
