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
        let lua = create_sandboxed_lua(
            &dir,
            &crate::plugin::load_options::PluginLoadOptions::new(name, trusted, false),
            tx,
        )
        .expect("sandbox lua");
        (lua, dir)
    }

    fn eval_module(lua: &Lua, dir: &Path) -> Table {
        let src = std::fs::read_to_string(dir.join("main.lua")).expect("main.lua");
        lua.load(&src)
            .set_name("main.lua")
            .eval()
            .unwrap_or_else(|e| panic!("loading {}: {e}", dir.display()))
    }

    fn call_run(module: &Table, arg: impl mlua::IntoLuaMulti) -> Table {
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
    fn keymap_command_reads_the_keymap_and_gets_its_command() {
        let (lua, dir) = load_plugin("keymap_command", false);
        let module = eval_module(&lua, &dir);
        let result = call_run(&module, ());
        assert_ok(&result, "keymap_command");
        let entry: Function = module.get("entry").unwrap();
        let args = lua.create_table().unwrap();
        args.set(1, "legacy").unwrap();
        args.set("command", "legacy").unwrap();
        entry.call::<()>((module.clone(), args)).unwrap();
        let received: String = module.get("received").unwrap();
        assert_eq!(received, "legacy");
    }

    #[test]
    fn roadmap_api_plugin_from_an_untrusted_plugin() {
        let outside = tempfile::tempdir().unwrap();
        let (lua, dir) = load_plugin("roadmap_api", false);
        let source = std::fs::read_to_string(dir.join("main.lua")).unwrap();
        let annotations = crate::plugin::annotations::Annotations::parse(&source);
        assert!(annotations.is_sync("entry"));
        annotations.check_since(LUA_API_VERSION).unwrap();
        let module = eval_module(&lua, &dir);
        let result = call_run(&module, outside.path().to_string_lossy().as_ref());
        assert_ok(&result, "roadmap_api");
    }

    #[test]
    fn keymap_command_manifest_declares_its_keys() {
        let manifest = crate::plugin::loader::PluginManifest::read_from_dir(
            &acceptance_root().join("keymap_command"),
        )
        .expect("manifest");
        let commands = manifest.keymap_commands();
        let ids: Vec<String> = commands.iter().map(|c| c.keymap_id()).collect();
        assert_eq!(
            ids,
            [
                "plugin.acceptance-keymap.hop",
                "plugin.acceptance-keymap.legacy"
            ]
        );
        assert_eq!(commands[0].keys_for("neovim"), Some("<leader>ph"));
        assert_eq!(commands[1].keys_for("norton"), Some("Ctrl+Alt+j"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fzf_port_pipes_candidates_through_a_picker() {
        let (lua, dir) = load_plugin("fzf", true);
        let module = eval_module(&lua, &dir);
        let pick: Function = module.get("pick").unwrap();
        let lines = lua.create_sequence_from(["b", "c", "a"]).unwrap();
        // `sort` reads lines on stdin and prints them, like a picker.
        let (picked, success): (Vec<String>, bool) =
            pick.call_async(("sort", lines)).await.unwrap();
        assert!(success);
        assert_eq!(picked, ["a", "b", "c"]);
        let target: Function = module.get("target").unwrap();
        let folder: String = target.call(("/srv/www/index.html", false)).unwrap();
        assert!(folder.ends_with("www"), "{folder}");
        let args: Function = module.get("args").unwrap();
        let fzf_args: Vec<String> = args.call("fzf").unwrap();
        assert_eq!(fzf_args[0], "--multi");
    }

    #[test]
    fn zoxide_port_ranks_host_visits_and_ignores_fake_ones() {
        let (lua, dir) = load_plugin("zoxide", false);
        let module = eval_module(&lua, &dir);
        lua.load("pairee.state.clear()").exec().unwrap();
        module
            .get::<Function>("setup")
            .unwrap()
            .call::<()>(())
            .unwrap();
        lua.load(
            r#"local on_cd = pairee.ps._callbacks.on_cd[1]
               local host = { from_host = true }
               on_cd({ path = "/work/projects" }, host)
               on_cd({ path = "/work/projects" }, host)
               on_cd({ path = "/old/projects-2019" }, host)
               -- Another plugin pretending to be Pairee does not count.
               for _ = 1, 5 do
                   on_cd({ path = "/evil/projects" }, { from_host = false, sender = "evil" })
               end"#,
        )
        .exec()
        .unwrap();
        let query: Function = module.get("query").unwrap();
        let best: Option<String> = query.call((vec!["PROJ"], "/elsewhere")).unwrap();
        assert_eq!(best.as_deref(), Some("/work/projects"));
        let other: Option<String> = query.call((vec!["proj"], "/work/projects")).unwrap();
        assert_eq!(other.as_deref(), Some("/old/projects-2019"));
        let none: Option<String> = query.call((vec!["evil"], "/")).unwrap();
        assert_eq!(none, None);
        lua.load("pairee.state.clear()").exec().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn code_preview_port_places_its_widgets() {
        use crate::app::state::popup::plugin_widget::Renderable;
        crate::ui::quickview::install_layout_solver();
        let (lua, dir) = load_plugin("code_preview", false);
        let module = eval_module(&lua, &dir);
        let key = lua.create_registry_value(module).unwrap();
        let source = std::fs::read_to_string(dir.join("main.lua")).unwrap();
        let (tx, _rx) = mpsc::channel::<PluginRequest>(8);
        let worker = crate::plugin::worker::Worker::new(
            "code_preview".into(),
            lua,
            key,
            crate::plugin::annotations::Annotations::parse(&source),
            tx,
        );
        let files = tempfile::tempdir().unwrap();
        let file = files.path().join("main.rs");
        std::fs::write(&file, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
        let job = crate::plugin::registry::PreviewJob {
            file_path: file,
            area_width: 40,
            area_height: 10,
            skip: 0,
        };

        let widget = worker.peek(job).await.expect("a preview");

        let Renderable::Stack(children) = widget else {
            panic!("{widget:?}")
        };
        assert_eq!(children.len(), 5);
        let Renderable::Placed(code_area, ref code) = children[4] else {
            panic!("{:?}", children[4])
        };
        assert_eq!((code_area.x, code_area.y, code_area.h), (1, 2, 8));
        assert!(matches!(**code, Renderable::Paragraph(_)));
        assert!(matches!(children[3], Renderable::Placed(rule, _) if rule.y == 1));
    }

    #[test]
    fn acceptance_tree_has_manifests() {
        for name in [
            "surface",
            "fs_roundtrip",
            "cx_utils",
            "command_echo",
            "keymap_command",
            "roadmap_api",
            "fzf",
            "zoxide",
            "code_preview",
        ] {
            let dir = acceptance_root().join(name);
            assert!(dir.join("manifest.toml").is_file(), "{name} manifest");
            assert!(dir.join("main.lua").is_file(), "{name} main.lua");
        }
    }
}
