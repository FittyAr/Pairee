use super::*;
use crate::plugin::manager::snapshot::{
    FileEntrySnapshot, TabSnapshot, TaskSnapshot, YankedSnapshot,
};
use mlua::Lua;

fn file(name: &str) -> FileEntrySnapshot {
    FileEntrySnapshot {
        name: name.into(),
        url: format!("/left/{name}"),
        path: format!("/left/{name}"),
        size: 4,
        is_dir: false,
        is_symlink: false,
        mime: "text/plain".into(),
        mtime: None,
        is_hidden: false,
        is_exec: false,
    }
}

fn sample_snapshot() -> AppStateSnapshot {
    AppStateSnapshot {
        active_panel: "left".into(),
        left_cwd: "/left".into(),
        right_cwd: "/srv".into(),
        hovered_file: Some(file("a.txt")),
        selected_files: vec![file("b.txt")],
        left_url: "/left".into(),
        right_url: "sftp://me@box:22/srv".into(),
        tabs: vec![
            TabSnapshot {
                url: "/left".into(),
                name: None,
                locked: false,
            },
            TabSnapshot {
                url: "/other".into(),
                name: Some("Work".into()),
                locked: true,
            },
        ],
        active_tab: 1,
        tasks: vec![TaskSnapshot {
            id: "j1".into(),
            kind: "copy".into(),
            status: "transferring".into(),
            sources: 2,
            destination: "/dst".into(),
            files_total: 10,
            files_done: 3,
            bytes_total: 100,
            bytes_done: 30,
        }],
        yanked: Some(YankedSnapshot {
            cut: true,
            urls: vec!["/left/a.txt".into(), "sftp://me@box:22/srv/x".into()],
        }),
    }
}

fn lua_with(snapshot: &AppStateSnapshot) -> Lua {
    let lua = Lua::new();
    let pairee = lua.create_table().unwrap();
    lua.globals().set("pairee", pairee).unwrap();
    install_snapshot(&lua, snapshot).unwrap();
    lua
}

#[test]
fn install_exposes_cwd_and_file_userdata() {
    let lua = lua_with(&sample_snapshot());
    let (cwd, name, selected_n, nested, right): (String, String, i64, String, String) = lua
        .load(
            "local cx = pairee.cx
             return cx.active.cwd, cx.active.hovered.name, #cx.active.selected,
                    cx.active.current.hovered.path, cx.right.cwd",
        )
        .eval()
        .unwrap();
    assert_eq!(cwd, "/left");
    assert_eq!(name, "a.txt");
    assert_eq!(selected_n, 1);
    assert_eq!(nested, "/left/a.txt");
    assert_eq!(right, "/srv");
}

#[test]
fn urls_tabs_tasks_and_yanked_are_exposed() {
    let lua = lua_with(&sample_snapshot());
    let (scheme, domain, tabs, idx, name, locked): (String, String, i64, i64, String, bool) = lua
        .load(
            "local cx = pairee.cx
             return cx.right.url.scheme, cx.right.url.domain, #cx.tabs, cx.tabs.idx,
                    cx.tabs[2].name, cx.tabs[2].locked",
        )
        .eval()
        .unwrap();
    assert_eq!((scheme.as_str(), domain.as_str()), ("sftp", "me@box:22"));
    assert_eq!((tabs, idx), (2, 2));
    assert_eq!((name.as_str(), locked), ("Work", true));
    let (kind, done, total, yanked, cut, remote): (String, u64, u64, i64, bool, String) = lua
        .load(
            "local cx = pairee.cx
             return cx.tasks[1].kind, cx.tasks[1].bytes_done, cx.tasks[1].bytes_total,
                    #cx.yanked, cx.yanked.is_cut, cx.yanked[2].scheme",
        )
        .eval()
        .unwrap();
    assert_eq!((kind.as_str(), done, total), ("copy", 30, 100));
    assert_eq!((yanked, cut, remote.as_str()), (2, true, "sftp"));
}

#[test]
fn bind_empty_has_safe_defaults() {
    let lua = Lua::new();
    let pairee = lua.create_table().unwrap();
    bind_empty(&lua, &pairee).unwrap();
    lua.globals().set("pairee", pairee).unwrap();
    let (cwd, hovered_nil, tabs, tasks, yanked, cut): (String, bool, i64, i64, i64, bool) = lua
        .load(
            "local cx = pairee.cx
             return cx.active.cwd, cx.active.hovered == nil, #cx.tabs, #cx.tasks,
                    #cx.yanked, cx.yanked.is_cut",
        )
        .eval()
        .unwrap();
    assert_eq!(cwd, "");
    assert!(hovered_nil);
    assert_eq!((tabs, tasks, yanked, cut), (0, 0, 0, false));
}
