use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::manager::PluginRequest;
use crate::plugin::runtime::bindings::fs::contain;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn create_sandboxed_lua(
    plugin_dir: &Path,
    options: &PluginLoadOptions,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<mlua::Lua> {
    let trusted = options.trusted;
    // Only a trusted plugin outside Secure Mode gets the full standard
    // library: `io`, `os.execute` or `package.loadlib` would bypass the
    // `pairee.fs` jail and the process allowlist of Secure Mode.
    let unrestricted = trusted && !options.secure_mode;
    // 1. Determine standard libraries to load
    let std_libs = if unrestricted {
        // ALL includes `debug`, which `new_with` rejects. Trusted plugins
        // get io/os/package via ALL_SAFE (no debug/ffi).
        mlua::StdLib::ALL_SAFE
    } else if trusted {
        // Secure Mode: coroutines and the clock part of `os` (below).
        restricted_libs() | mlua::StdLib::COROUTINE | mlua::StdLib::OS
    } else {
        restricted_libs()
    };

    let lua = mlua::Lua::new_with(std_libs, mlua::LuaOptions::default())?;
    crate::plugin::limits::apply(&lua, trusted);

    // 2. Sandboxing restrictions (untrusted plugins and Secure Mode)
    if !unrestricted {
        let globals = lua.globals();
        // Remove dangerous global evaluation/loading functions
        for name in ["load", "loadstring", "dofile", "loadfile"] {
            globals.set(name, mlua::Value::Nil)?;
        }
        if trusted {
            keep_clock_os(&lua)?;
        }
    }

    // 3. Plugin-relative require (the same for trusted and untrusted)
    setup_require(&lua, plugin_dir)?;

    // 4. Bind the pairee global namespace
    crate::plugin::runtime::standard::bind_runtime(&lua, plugin_dir, options, tx)?;

    Ok(lua)
}

/// The standard libraries every plugin gets.
fn restricted_libs() -> mlua::StdLib {
    mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH
}

/// `os` functions that only read the clock; Secure Mode keeps these and
/// drops the rest (`execute`, `remove`, `rename`, `exit`, `getenv`,
/// `tmpname`, `setlocale`).
const CLOCK_OS_FUNCTIONS: [&str; 4] = ["clock", "date", "difftime", "time"];

/// Replaces `os` with a table holding only [`CLOCK_OS_FUNCTIONS`].
fn keep_clock_os(lua: &mlua::Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    let full: mlua::Table = globals.get("os")?;
    let os = lua.create_table()?;
    for name in CLOCK_OS_FUNCTIONS {
        os.set(name, full.get::<mlua::Value>(name)?)?;
    }
    globals.set("os", os)
}

/// `a.b` → `a/b`, or `None` for names that are not plain dotted
/// identifiers (no separators, drive letters or empty / `..` segments).
fn module_rel_path(module: &str) -> Option<String> {
    let valid = module.split('.').all(crate::plugin::updater::is_identifier);
    valid.then(|| module.replace('.', "/"))
}

/// Installs `require(name)`: loads `<plugin>/<name>.lua` (dots become
/// folders) or `<plugin>/<name>/init.lua`, runs it once and caches the
/// result in a table private to this plugin. It works without the
/// `package` library (untrusted plugins have none) and resolves paths with
/// the same containment as `pairee.fs` (no `..`, symlink or junction can
/// leave the plugin folder).
fn setup_require(lua: &mlua::Lua, plugin_dir: &Path) -> mlua::Result<()> {
    let root = plugin_dir.to_path_buf();
    let loaded = Arc::new(lua.create_registry_value(lua.create_table()?)?);

    let require_fn =
        lua.create_function(move |lua, module: String| -> mlua::Result<mlua::Value> {
            let cache: mlua::Table = lua.registry_value(&loaded)?;
            let cached: mlua::Value = cache.get(module.as_str())?;
            if !cached.is_nil() {
                return Ok(cached);
            }
            let rel = module_rel_path(&module).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("require: invalid module name {module:?}"))
            })?;
            let candidates = [format!("{rel}.lua"), format!("{rel}/init.lua")];
            let (chunk_name, code) = candidates
                .iter()
                .find_map(|candidate| {
                    let target = contain(&root, &root.join(candidate))?;
                    let code = target.read_to_string().ok()?;
                    Some((candidate.clone(), code))
                })
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!(
                        "require: module {module:?} not found inside the plugin folder"
                    ))
                })?;

            let value: mlua::Value = lua
                .load(&code)
                .set_name(chunk_name)
                .into_function()?
                .call(module.as_str())?;
            let value = if value.is_nil() {
                mlua::Value::Boolean(true)
            } else {
                value
            };
            cache.set(module.as_str(), value.clone())?;
            Ok(value)
        })?;

    lua.globals().set("require", require_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn options(trusted: bool) -> PluginLoadOptions {
        PluginLoadOptions::new("demo", trusted, false)
    }

    #[tokio::test]
    async fn test_create_sandboxed_lua_restrictions() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let lua = create_sandboxed_lua(dir.path(), &options(false), tx).unwrap();

        // Standard restricted globals should be nil
        let globals = lua.globals();
        assert!(globals.get::<mlua::Value>("load").unwrap().is_nil());
        assert!(globals.get::<mlua::Value>("dofile").unwrap().is_nil());

        // io module should not exist
        assert!(globals.get::<mlua::Value>("io").unwrap().is_nil());
        // os module should not exist
        assert!(globals.get::<mlua::Value>("os").unwrap().is_nil());
    }

    #[test]
    fn trusted_lua_loads_without_debug() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), &options(true), tx).unwrap();
        let globals = lua.globals();
        assert!(!globals.get::<mlua::Value>("pairee").unwrap().is_nil());
        assert!(!globals.get::<mlua::Value>("io").unwrap().is_nil());
    }

    #[test]
    fn secure_mode_trusted_lua_has_no_io_os_or_loaders() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let options = PluginLoadOptions::new("demo", true, true);
        let lua = create_sandboxed_lua(dir.path(), &options, tx).unwrap();
        let globals = lua.globals();
        for name in ["io", "package", "load", "loadfile", "dofile"] {
            assert!(globals.get::<mlua::Value>(name).unwrap().is_nil(), "{name}");
        }
        let (execute, getenv, time, coroutine): (bool, bool, bool, bool) = lua
            .load(
                "return os.execute == nil, os.getenv == nil, \
                 type(os.time()) == 'number', type(coroutine.wrap) == 'function'",
            )
            .eval()
            .unwrap();
        assert!(execute && getenv && time && coroutine);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn untrusted_fs_is_jailed_without_secure_mode() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("pwn.txt");
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), &options(false), tx).unwrap();
        lua.globals()
            .set("target", target.to_string_lossy().to_string())
            .unwrap();
        let err = lua
            .load(r#"pairee.fs.write(target, "x")"#)
            .exec()
            .unwrap_err();
        assert!(err.to_string().contains("Security violation"), "{err}");
        assert!(!target.exists());
    }

    #[test]
    fn secure_mode_flag_is_not_lua_writable() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("pwn.txt");
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), &options(false), tx).unwrap();
        lua.globals()
            .set("target", target.to_string_lossy().to_string())
            .unwrap();
        let result = lua
            .load(r#"pairee._secure_mode = false; pairee.fs.write(target, "x")"#)
            .exec();
        assert!(result.is_err());
        assert!(!target.exists());
    }

    fn plugin_with_modules() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("lib").join("pkg")).unwrap();
        std::fs::write(
            dir.path().join("lib").join("util.lua"),
            "COUNT = (COUNT or 0) + 1 local name = ... return { name = name, n = COUNT }",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("lib").join("pkg").join("init.lua"),
            "return 'pkg'",
        )
        .unwrap();
        std::fs::write(dir.path().join("side.lua"), "SIDE = true").unwrap();
        dir
    }

    #[test]
    fn require_works_for_untrusted_and_trusted_plugins() {
        for trusted in [false, true] {
            let dir = plugin_with_modules();
            let (tx, _rx) = tokio::sync::mpsc::channel(4);
            let lua = create_sandboxed_lua(dir.path(), &options(trusted), tx).unwrap();
            let (name, n, again, pkg, side): (String, i64, i64, String, bool) = lua
                .load(
                    "local a = require('lib.util') local b = require('lib.util') \
                     return a.name, a.n, b.n, require('lib.pkg'), require('side')",
                )
                .eval()
                .unwrap();
            assert_eq!(name, "lib.util");
            assert_eq!((n, again), (1, 1), "module runs once (trusted={trusted})");
            assert_eq!(pkg, "pkg");
            assert!(side);
        }
    }

    #[test]
    fn require_cannot_leave_the_plugin_folder() {
        let parent = tempdir().unwrap();
        let plugin = parent.path().join("demo.pairee");
        std::fs::create_dir(&plugin).unwrap();
        std::fs::write(parent.path().join("evil.lua"), "return 'escaped'").unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(&plugin, &options(false), tx).unwrap();
        for name in [
            "..evil",
            "../evil",
            "..",
            "/etc/passwd",
            "a..b",
            "",
            "C:evil",
        ] {
            let result = lua
                .load(format!("return require({name:?})"))
                .eval::<mlua::Value>();
            assert!(result.is_err(), "{name:?} should be refused");
        }
        let err = lua.load("return require('missing')").exec().unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn require_refuses_symlinks_out_of_the_plugin() {
        let parent = tempdir().unwrap();
        let plugin = parent.path().join("demo.pairee");
        std::fs::create_dir(&plugin).unwrap();
        std::fs::write(parent.path().join("evil.lua"), "return 'escaped'").unwrap();
        std::os::unix::fs::symlink(parent.path().join("evil.lua"), plugin.join("link.lua"))
            .unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(&plugin, &options(false), tx).unwrap();
        assert!(lua.load("return require('link')").exec().is_err());
    }
}
