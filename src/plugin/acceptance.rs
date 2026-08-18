//! CI acceptance: load the Lua plugins under `tests/plugin_acceptance/`.
//!
//! These are real plugin trees (manifest + `main.lua`) that exercise the
//! public `pairee.*` surface. `cargo test` (and the Check workflow) runs them.

#[cfg(test)]
mod tests {
    use crate::plugin::manager::PluginRequest;
    use crate::plugin::runtime::api_version::LUA_API_VERSION;
    use crate::plugin::sandbox::create_sandboxed_lua;
    use mlua::{Function, Lua, Table};
    use std::path::{Path, PathBuf};
    use tokio::sync::mpsc;

    fn acceptance_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/plugin_acceptance")
    }

    fn load_plugin(name: &str, trusted: bool) -> (Lua, PathBuf) {
        let dir = acceptance_root().join(name);
        assert!(
            dir.join("main.lua").is_file(),
            "missing acceptance plugin {name} at {}",
            dir.display()
        );
        let (tx, _rx) = mpsc::channel::<PluginRequest>(8);
        let lua = create_sandboxed_lua(&dir, trusted, tx).expect("sandbox lua");
        (lua, dir)
    }

    fn eval_module<'lua>(lua: &'lua Lua, dir: &Path) -> Table<'lua> {
        let src = std::fs::read_to_string(dir.join("main.lua")).expect("main.lua");
        lua.load(&src)
            .set_name("main.lua")
            .eval()
            .unwrap_or_else(|e| panic!("loading {}: {e}", dir.display()))
    }

    fn call_run<'lua>(module: &Table<'lua>, arg: impl mlua::IntoLuaMulti<'lua>) -> Table<'lua> {
        let run: Function = module.get("run").expect("M.run");
        run.call(arg)
            .unwrap_or_else(|e| panic!("M.run failed: {e}"))
    }

    fn assert_ok(result: &Table, ctx: &str) {
        let ok: bool = result.get("ok").unwrap_or(false);
        if !ok {
            let extra: String = result.get("missing").unwrap_or_default();
            panic!("{ctx} returned ok=false missing={extra}");
        }
    }

    #[test]
    fn surface_lists_stable_bindings() {
        let (lua, dir) = load_plugin("surface", true);
        let module = eval_module(&lua, &dir);
        let result = call_run(&module, ());
        assert_ok(&result, "surface");
        let api: String = result.get("api").unwrap();
        assert_eq!(api, LUA_API_VERSION);
    }

    #[test]
    fn fs_roundtrip_plugin() {
        let root = tempfile::tempdir().unwrap();
        let (lua, dir) = load_plugin("fs_roundtrip", true);
        let module = eval_module(&lua, &dir);
        let result = call_run(&module, root.path().to_string_lossy().as_ref());
        assert_ok(&result, "fs_roundtrip");
        let copied: u64 = result.get("copied").unwrap();
        assert_eq!(copied, 5);
    }

    #[test]
    fn cx_and_utils_plugin() {
        let (lua, dir) = load_plugin("cx_utils", true);
        let module = eval_module(&lua, &dir);
        let result = call_run(&module, ());
        assert_ok(&result, "cx_utils");
        let family: String = result.get("family").unwrap();
        assert!(family == "windows" || family == "unix" || family == "wasm");
    }

    #[test]
    fn command_builder_surface_from_lua() {
        let (lua, dir) = load_plugin("command_echo", true);
        let module = eval_module(&lua, &dir);
        let surface: Function = module.get("surface").unwrap();
        let result: Table = surface.call(()).unwrap();
        assert_ok(&result, "command_echo.surface");
    }

    #[tokio::test]
    async fn command_echo_output_from_lua() {
        let (lua, dir) = load_plugin("command_echo", true);
        let module = eval_module(&lua, &dir);
        let run: Function = module.get("run").unwrap();
        let result: Table = run
            .call_async(())
            .await
            .unwrap_or_else(|e| panic!("command_echo M.run: {e}"));
        assert_ok(&result, "command_echo");
        let stdout: String = result.get("stdout").unwrap();
        assert!(
            stdout.to_ascii_lowercase().contains("hello"),
            "stdout was {stdout:?}"
        );
    }

    #[test]
    fn acceptance_tree_has_manifests() {
        for name in ["surface", "fs_roundtrip", "cx_utils", "command_echo"] {
            let dir = acceptance_root().join(name);
            assert!(dir.join("manifest.toml").is_file(), "{name} manifest");
            assert!(dir.join("main.lua").is_file(), "{name} main.lua");
        }
    }
}
