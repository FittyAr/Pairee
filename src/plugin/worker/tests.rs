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
                    ..Default::default()
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
    w.event("on_cd", r#"{"path":"/x"}"#, None).await;
    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    assert_eq!(table.get::<String>("path").unwrap(), "/x");
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribers_see_who_sent_an_event() {
    let dir = tempfile::tempdir().unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = { seen = {} }
        local function record(_, meta)
            M.seen[#M.seen + 1] = tostring(meta.from_host) .. ":" .. tostring(meta.sender)
                .. ":" .. tostring(meta.trusted)
        end
        pairee.ps.sub("on_cd", record)
        pairee.ps.sub("done", record)
        return M
        "#,
        false,
    );
    let other = crate::plugin::manager::PluginOrigin {
        name: "other".into(),
        trusted: false,
    };
    w.event("on_cd", "{}", None).await;
    w.event("done", "{}", Some(&other)).await;

    let table: Table = w.lua.registry_value(&w.table_key).unwrap();
    let seen: Vec<String> = table
        .get::<Table>("seen")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(seen, ["true:nil:nil", "false:other:false"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn untrusted_plugins_cannot_publish_reserved_events() {
    let dir = tempfile::tempdir().unwrap();
    let source = r#"
        local M = {}
        function M:entry()
            M.host = pcall(pairee.ps.pub, "on_cd", { path = "/fake" })
            M.system = pcall(pairee.ps.pub, "pairee.quit", {})
            M.own = pcall(pairee.ps.pub, "my-event", {})
        end
        return M
        "#;
    for (trusted, reserved_allowed) in [(false, false), (true, true)] {
        let (w, _rx) = worker(dir.path(), source, trusted);
        w.entry(Vec::new()).await;
        let table: Table = w.lua.registry_value(&w.table_key).unwrap();
        assert_eq!(table.get::<bool>("host").unwrap(), reserved_allowed);
        assert_eq!(table.get::<bool>("system").unwrap(), reserved_allowed);
        assert!(table.get::<bool>("own").unwrap());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn peek_can_show_the_previewed_image() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let file = outside.path().join("pic.png");
    image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]))
        .save(&file)
        .unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        function M:peek(job) return pairee.image.show(job.file) end
        return M
        "#,
        false,
    );
    let widget = w.peek(job(&file, 0)).await.expect("widget");
    let Renderable::Image(image) = widget else {
        panic!("{widget:?}");
    };
    assert_eq!(image.path, file);
    assert_eq!(image.image.expect("decoded").width(), 4);
    // Outside a previewer call the same file is not showable.
    w.lua
        .globals()
        .set("file", file.to_string_lossy().as_ref())
        .unwrap();
    let shown: mlua::Value = w.lua.load("return pairee.image.show(file)").eval().unwrap();
    assert!(shown.is_nil());
}

#[tokio::test(flavor = "multi_thread")]
async fn metamethods_run_by_the_host_are_under_the_watchdog() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "x").unwrap();
    let (w, _rx) = worker(
        dir.path(),
        r#"
        local spin = { __index = function() while true do end end }
        pairee.ps._callbacks = setmetatable({}, spin)
        return setmetatable({}, spin)
        "#,
        false,
    );
    crate::plugin::limits::set_test_budget(&w.lua, std::time::Duration::from_millis(200));
    let started = std::time::Instant::now();
    w.entry(vec!["go".into()]).await;
    assert!(w.peek(job(&file, 0)).await.is_none());
    w.event("x", "{}", None).await;
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread")]
async fn returned_values_are_converted_under_the_watchdog() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "x").unwrap();
    let (mut w, _rx) = worker(
        dir.path(),
        r#"
        local M = {}
        local spin = function() while true do end end
        function M:peek() return setmetatable({}, { __index = spin }) end
        function M:preload() return false, setmetatable({}, { __tostring = spin }) end
        return M
        "#,
        false,
    );
    crate::plugin::limits::set_test_budget(&w.lua, std::time::Duration::from_millis(200));
    let started = std::time::Instant::now();
    assert!(w.peek(job(&file, 0)).await.is_none());
    assert!(!w.preload(job(&file, 0)).await);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn an_image_shown_many_times_is_decoded_once() {
    use crate::app::state::popup::plugin_widget::PluginImage;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.png");
    image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]))
        .save(&file)
        .unwrap();
    let node = || {
        Renderable::Image(PluginImage {
            path: file.clone(),
            image: None,
        })
    };
    let mut widget = Renderable::Stack((0..100).map(|_| node()).collect());
    decode_images(&mut widget);
    let images = widget.images_mut();
    let first = images[0].image.clone().expect("decoded");
    assert!(images.iter().all(|i| i
        .image
        .as_ref()
        .is_some_and(|img| std::sync::Arc::ptr_eq(img, &first))));
}
