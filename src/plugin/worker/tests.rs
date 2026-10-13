//! Worker calls driven from Lua plugin sources.

use super::*;
use crate::app::state::popup::plugin_widget::{Renderable, WTextBody};
use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::manager::snapshot::AppStateSnapshot;
use crate::plugin::registry::PreviewJob;
use crate::plugin::sandbox::create_sandboxed_lua;
use std::path::Path;

/// A worker for the plugin whose `main.lua` is `source`, in `dir`, plus
/// the receiver of its requests to the main loop.
fn worker(dir: &Path, source: &str, trusted: bool) -> (Worker, mpsc::Receiver<PluginRequest>) {
    std::fs::write(dir.join("main.lua"), source).unwrap();
    let (tx, rx) = mpsc::channel(8);
    let options = PluginLoadOptions::new("demo", trusted, false);
    let lua = create_sandboxed_lua(dir, &options, tx.clone()).unwrap();
    let table: Table = lua.load(source).eval().unwrap();
    let key = lua.create_registry_value(table).unwrap();
    let annotations = Annotations::parse(source);
    (Worker::new("demo".into(), lua, key, annotations, tx), rx)
}

fn job(path: &Path, skip: usize) -> PreviewJob {
    PreviewJob {
        file_path: path.to_path_buf(),
        area_width: 40,
        area_height: 10,
        skip,
    }
}

/// Answers every snapshot request with `cwd` as the left panel folder.
fn fake_main_loop(mut rx: mpsc::Receiver<PluginRequest>, cwd: &str) {
    let cwd = cwd.to_string();
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            if let PluginRequest::GetStateSnapshot(reply) = req {
                let _ = reply.send(AppStateSnapshot {
                    active_panel: "left".into(),
                    left_cwd: cwd.clone(),
                    right_cwd: String::new(),
                    hovered_file: None,
                    selected_files: Vec::new(),
                });
            }
        }
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn entry_can_await_async_bindings() {
    let dir = tempfile::tempdir().unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        function M:entry(args)
            pairee.utils.sleep(0)
            M.got = args.command .. ":" .. args[2]
        end
        return M
        "#,
        false,
    );
    w.entry(vec!["go".into(), "x".into()]).await;
    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    assert_eq!(table.get::<String>("got").unwrap(), "go:x");
}

#[tokio::test(flavor = "multi_thread")]
async fn peek_gets_a_file_userdata_and_area() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("notes.md");
    std::fs::write(&file, "hello").unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        function M:peek(job)
            return pairee.ui.Paragraph(job.file.name .. " " .. job.file.mime .. " " .. job.area.w .. "x" .. job.area.h .. " " .. job.skip)
        end
        return M
        "#,
        false,
    );
    let widget = w.peek(job(&file, 3)).await.expect("widget");
    let Renderable::Paragraph(p) = widget else {
        panic!("{widget:?}");
    };
    let WTextBody::Lines(lines) = p.text.body else {
        panic!("ansi");
    };
    assert_eq!(lines[0].spans[0].content, "notes.md text/plain 40x10 3");
}

#[tokio::test(flavor = "multi_thread")]
async fn completed_preloads_are_not_repeated() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "x").unwrap();
    let (mut w, _rx) = worker(
        dir.path(),
        r#"
        local M = { calls = 0 }
        function M:preload(job)
            M.calls = M.calls + 1
            return M.calls > 1
        end
        return M
        "#,
        false,
    );
    assert!(
        !w.preload(job(&file, 0)).await,
        "first call reports incomplete"
    );
    assert!(w.preload(job(&file, 0)).await);
    assert!(w.preload(job(&file, 0)).await, "cached");
    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    assert_eq!(table.get::<i64>("calls").unwrap(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn seek_returns_the_new_skip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "x").unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        function M:seek(job) return job.skip + job.units end
        return M
        "#,
        false,
    );
    assert_eq!(w.seek(job(&file, 10), 5).await, Some(15));
    assert_eq!(w.seek(job(&file, 2), -9).await, Some(0));
    let (none, _rx) = worker(dir.path(), "return {}", false);
    assert_eq!(none.seek(job(&file, 2), 1).await, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_annotated_methods_see_a_fresh_context() {
    let dir = tempfile::tempdir().unwrap();
    let (w, rx) = worker(
        dir.path(),
        r#"--- @sync entry
        local M = {}
        function M:entry() M.cwd = pairee.cx.active.cwd end
        return M
        "#,
        false,
    );
    fake_main_loop(rx, "/fresh");
    w.entry(Vec::new()).await;
    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    assert_eq!(table.get::<String>("cwd").unwrap(), "/fresh");
}

#[tokio::test(flavor = "multi_thread")]
async fn hook_callbacks_receive_the_payload() {
    let dir = tempfile::tempdir().unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        pairee.ps.sub("on_cd", function(data) M.path = data.path end)
        return M
        "#,
        false,
    );
    w.event("on_cd", r#"{"path":"/x"}"#).await;
    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    assert_eq!(table.get::<String>("path").unwrap(), "/x");
}
