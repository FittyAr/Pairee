//! `pairee.fs` on a remote URL of a connection "open in a panel" (a test
//! WebDAV server), per trust level.

use crate::config::settings::RemoteKind;
use crate::fs::remote::tests::served_folder;
use crate::fs::remote::{RemoteConn, RemoteInfo};
use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::manager::PluginRequest;
use crate::plugin::sandbox::create_sandboxed_lua;
use mlua::Table;
use tokio::sync::mpsc;

/// A plugin Lua state whose main loop answers connection lookups with the
/// WebDAV connection at `addr`.
fn plugin(trusted: bool, secure: bool, conn: RemoteConn) -> mlua::Lua {
    let dir = tempfile::tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel::<PluginRequest>(8);
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            if let PluginRequest::RemoteConnection {
                scheme,
                domain,
                reply,
            } = req
            {
                let found = conn.info().matches(&scheme, &domain).then(|| conn.clone());
                let _ = reply.send(found);
            }
        }
    });
    let options = PluginLoadOptions::new("remote", trusted, secure);
    create_sandboxed_lua(dir.path(), &options, tx).unwrap()
}

fn webdav() -> (tempfile::TempDir, RemoteConn, String) {
    let (dir, addr, folder, _) = served_folder(true);
    let url = format!("webdav://{addr}/");
    let remote = crate::fs::remote::connect(&url).unwrap();
    let info = RemoteInfo {
        kind: RemoteKind::WebDav,
        host: addr.ip().to_string(),
        port: Some(addr.port()),
        username: String::new(),
    };
    let base = format!("{}/{folder}", info.url_prefix());
    (dir, RemoteConn::new(remote.vfs, info), base)
}

async fn run(lua: &mlua::Lua, base: &str, code: &str) -> mlua::Result<Table> {
    lua.globals().set("base", base)?;
    let chunk = format!("return function() local fs = pairee.fs\n{code}\nend");
    let f: mlua::Function = lua.load(chunk).eval()?;
    f.call_async(()).await
}

#[tokio::test(flavor = "multi_thread")]
async fn trusted_plugins_read_and_write_open_connections() {
    let (dir, conn, base) = webdav();
    let lua = plugin(true, false, conn);
    let out = run(
        &lua,
        &base,
        r#"fs.mkdir("dir", base .. "/sub")
           fs.write(base .. "/sub/a.txt", "remote text")
           fs.rename(base .. "/sub/a.txt", pairee.Url(base .. "/sub/b.txt"))
           local names = {}
           for _, f in ipairs(fs.read_dir(base .. "/sub")) do names[#names + 1] = f.name end
           local cha = fs.cha(base .. "/sub/b.txt")
           local file = fs.file(base .. "/sub/b.txt")
           return { text = fs.read(base .. "/sub/b.txt"), names = table.concat(names, ","),
                    len = cha.len, exists = fs.exists(base .. "/sub/a.txt"),
                    url = file.url, missing = fs.stat(base .. "/nope") == nil }"#,
    )
    .await
    .unwrap();
    assert_eq!(out.get::<String>("text").unwrap(), "remote text");
    assert_eq!(out.get::<String>("names").unwrap(), "b.txt");
    assert_eq!(out.get::<u64>("len").unwrap(), 11);
    assert!(!out.get::<bool>("exists").unwrap());
    assert!(out.get::<bool>("missing").unwrap());
    assert!(out.get::<String>("url").unwrap().starts_with("webdav://"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("sub").join("b.txt")).unwrap(),
        "remote text"
    );

    run(
        &lua,
        &base,
        r#"fs.remove("dir_all", base .. "/sub") return {}"#,
    )
    .await
    .unwrap();
    assert!(!dir.path().join("sub").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn secure_mode_reads_only_and_untrusted_plugins_get_nothing() {
    let (dir, conn, base) = webdav();
    std::fs::write(dir.path().join("r.txt"), "read me").unwrap();

    let secure = plugin(true, true, conn.clone());
    let out = run(
        &secure,
        &base,
        r#"return { text = fs.read(base .. "/r.txt") }"#,
    )
    .await
    .unwrap();
    assert_eq!(out.get::<String>("text").unwrap(), "read me");
    let err = run(
        &secure,
        &base,
        r#"fs.write(base .. "/w.txt", "x") return {}"#,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("Security violation"), "{err}");
    assert!(!dir.path().join("w.txt").exists());

    let untrusted = plugin(false, false, conn);
    let err = run(&untrusted, &base, r#"return { t = fs.read_dir(base) }"#)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("untrusted"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn urls_of_closed_connections_and_remote_copies_are_refused() {
    let (_dir, conn, base) = webdav();
    let lua = plugin(true, false, conn);
    let err = run(
        &lua,
        "sftp://nobody@elsewhere",
        r#"return { t = fs.read(base .. "/x") }"#,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("no open connection"), "{err}");
    let err = run(
        &lua,
        &base,
        r#"fs.copy(base .. "/a", base .. "/b") return {}"#,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("local paths"), "{err}");
}
