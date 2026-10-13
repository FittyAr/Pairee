//! `pairee.preview_code` / `pairee.preview_widget` in a sandboxed runtime.

use super::*;
use crate::app::state::popup::plugin_widget::Renderable;
use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::sandbox::create_sandboxed_lua;
use std::path::Path;

fn runtime(dir: &Path) -> (Lua, mpsc::Receiver<PluginRequest>) {
    let (tx, rx) = mpsc::channel(4);
    let options = PluginLoadOptions::new("demo", false, false);
    (create_sandboxed_lua(dir, &options, tx).unwrap(), rx)
}

#[test]
fn preview_code_reads_only_the_previewed_file() {
    let plugin = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let file = outside.path().join("main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let binary = outside.path().join("blob.bin");
    std::fs::write(&binary, b"\0\x01\x02").unwrap();
    let (lua, _rx) = runtime(plugin.path());
    lua.globals()
        .set("file", file.to_string_lossy().as_ref())
        .unwrap();
    lua.globals()
        .set("blob", binary.to_string_lossy().as_ref())
        .unwrap();
    let kind: String = lua
        .load("local _, e = pairee.preview_code({ file = file }) return e.kind")
        .eval()
        .unwrap();
    assert_eq!(kind, "PermissionDenied");

    lua.set_app_data(PreviewTarget(file.clone()));
    let widget: Value = lua
        .load("return pairee.preview_code({ file = file, lines = 10 })")
        .eval()
        .unwrap();
    let Renderable::Paragraph(p) = renderable_arg(&widget).unwrap() else {
        panic!()
    };
    assert_eq!(p.text.height(), 1);
    lua.set_app_data(PreviewTarget(binary.clone()));
    let kind: String = lua
        .load("local _, e = pairee.preview_code({ file = blob }) return e.kind")
        .eval()
        .unwrap();
    assert_eq!(kind, "InvalidData");
}

#[tokio::test(flavor = "multi_thread")]
async fn preview_widget_updates_the_previewed_file() {
    let plugin = tempfile::tempdir().unwrap();
    let (lua, mut rx) = runtime(plugin.path());
    let target = PathBuf::from("shown.txt");
    lua.set_app_data(PreviewTarget(target.clone()));
    let f: mlua::Function = lua
        .load(r#"return function() return pairee.preview_widget({ skip = 2 }, pairee.ui.Text("late")) end"#)
        .eval()
        .unwrap();
    let ok: bool = f.call_async(()).await.unwrap();
    assert!(ok);
    let Some(PluginRequest::UpdatePluginWidget { path, preview }) = rx.recv().await else {
        panic!("no update");
    };
    assert_eq!(path, target);
    assert_eq!((preview.plugin.as_str(), preview.skip), ("demo", 2));

    // Without a file being previewed (and no opts.file) nothing is sent.
    lua.remove_app_data::<PreviewTarget>();
    let ok: bool = f.call_async(()).await.unwrap();
    assert!(!ok);
    assert!(rx.try_recv().is_err());
}
