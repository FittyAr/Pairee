use super::QUICK_VIEW_DEBOUNCE;
use super::cache::{PreviewCache, PreviewKey};
use super::load::{QuickViewPreview, load_preview, read_text_prefix};
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::config::localization::t;
use crate::fs::vfs::LocalVfs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

fn key(path: &str, size: u64, modified: Option<SystemTime>) -> PreviewKey {
    PreviewKey {
        path: PathBuf::from(path),
        modified,
        size,
        allow_image: true,
    }
}

fn preview(line: &str) -> Arc<QuickViewPreview> {
    Arc::new(QuickViewPreview {
        content: vec![line.to_string()],
        image: None,
    })
}

#[test]
fn cache_hits_only_same_file_version() {
    let mut cache = PreviewCache::default();
    let t0 = Some(SystemTime::UNIX_EPOCH);
    cache.insert(key("/a", 1, t0), preview("a"));
    assert!(cache.get(&key("/a", 1, t0)).is_some());
    assert!(cache.get(&key("/a", 2, t0)).is_none(), "size changed");
    let t1 = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1));
    assert!(cache.get(&key("/a", 1, t1)).is_none(), "mtime changed");
    cache.insert(key("/a", 1, t1), preview("a2"));
    assert_eq!(cache.len(), 1, "older version replaced");
}

#[test]
fn cache_evicts_least_recently_used() {
    let mut cache = PreviewCache::with_capacity(2);
    cache.insert(key("/a", 0, None), preview("a"));
    cache.insert(key("/b", 0, None), preview("b"));
    assert!(cache.get(&key("/a", 0, None)).is_some()); // /a now most recent
    cache.insert(key("/c", 0, None), preview("c"));
    assert!(cache.get(&key("/b", 0, None)).is_none());
    assert!(cache.get(&key("/a", 0, None)).is_some());
    assert!(cache.get(&key("/c", 0, None)).is_some());
}

#[test]
fn text_preview_is_capped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.txt");
    std::fs::write(&path, "line\n".repeat(1000)).unwrap();
    let p = load_preview(&LocalVfs, &path, false, 50);
    assert_eq!(p.content.iter().filter(|l| *l == "line").count(), 10);
    assert!(
        p.content.last().unwrap().starts_with('['),
        "truncation notice"
    );
}

#[test]
fn utf8_cut_by_cap_is_not_binary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("u.txt");
    std::fs::write(&path, "añb").unwrap(); // 'ñ' is 2 bytes: a,0xC3,0xB1,b
    assert_eq!(read_text_prefix(&LocalVfs, &path, 2).as_deref(), Some("a"));
    std::fs::write(&path, [0x61, 0x00, 0x62, 0x01, 0x02]).unwrap();
    assert_eq!(
        read_text_prefix(&LocalVfs, &path, 10),
        None,
        "NUL bytes = binary"
    );
}

#[test]
fn non_utf8_text_previews_in_its_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.txt");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(
        "hola
mundo
"
        .encode_utf16()
        .flat_map(u16::to_le_bytes),
    );
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        load_preview(&LocalVfs, &path, false, 1024).content,
        ["hola", "mundo"]
    );
    let (latin, _, _) = encoding_rs::WINDOWS_1252.encode(
        "Canción de la niña con su pingüino
",
    );
    std::fs::write(&path, latin).unwrap();
    assert_eq!(
        load_preview(&LocalVfs, &path, false, 1024).content,
        ["Canción de la niña con su pingüino"]
    );
}

fn quick_view_lines(state: &AppState) -> Option<Vec<String>> {
    match state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => Some(qv.content.clone()),
        _ => None,
    }
}

#[test]
fn preview_is_debounced_then_cached() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();
    std::fs::write(dir.path().join("b.txt"), "beta").unwrap();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    state.refresh_panel(ActivePanel::Left, false, true);
    state.panels.quick_view_active = true;
    let idx = |state: &AppState, name: &str| {
        state
            .panels
            .side(ActivePanel::Left)
            .entries
            .iter()
            .position(|e| e.name == name)
            .unwrap()
    };

    state.panels.side_mut(ActivePanel::Left).cursor_index = idx(&state, "a.txt");
    state.update_quick_view_images(false);
    assert_eq!(quick_view_lines(&state), Some(vec![t("quickview_loading")]));
    state.poll_quick_view();
    assert_eq!(
        quick_view_lines(&state),
        Some(vec![t("quickview_loading")]),
        "not loaded before the debounce elapses"
    );
    std::thread::sleep(QUICK_VIEW_DEBOUNCE + Duration::from_millis(30));
    assert!(state.poll_quick_view());
    assert_eq!(quick_view_lines(&state), Some(vec!["alpha".to_string()]));

    // Move away (loading) and back: the cached preview shows immediately.
    state.panels.side_mut(ActivePanel::Left).cursor_index = idx(&state, "b.txt");
    state.update_quick_view_images(false);
    state.panels.side_mut(ActivePanel::Left).cursor_index = idx(&state, "a.txt");
    state.update_quick_view_images(false);
    assert_eq!(quick_view_lines(&state), Some(vec!["alpha".to_string()]));
    assert!(state.quick_view.pending.is_none());
}

/// A quick view showing a plugin preview (`seekable` or not).
fn plugin_quick_view(seekable: bool) -> AppState {
    use crate::app::state::popup::plugin_widget::WText;
    use crate::app::state::popup::{PluginPreview, QuickViewDialog, Renderable};
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    state.panels.quick_view_active = true;
    state
        .dialogs
        .replace(PopupType::QuickViewPanel(Box::new(QuickViewDialog {
            path: PathBuf::from("a.txt"),
            content: vec![String::new(); 100],
            scroll: 0,
            image_data: None,
            plugin: Some(PluginPreview {
                widget: Renderable::text(WText::raw("x")),
                plugin: "demo".into(),
                skip: 0,
                seekable,
            }),
        })));
    state
}

fn page_down(state: &mut AppState) -> usize {
    use crossterm::event::{KeyCode, KeyEvent};
    let mut context = crate::app::context::AppContext::new(crate::config::AppConfig::default());
    let handler = state.dialogs.top().unwrap().dialog().key_handler();
    let _ = handler(state, KeyEvent::from(KeyCode::PageDown), &mut context);
    match state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => qv.scroll,
        _ => panic!("quick view closed"),
    }
}

#[test]
fn page_keys_go_to_a_seekable_plugin_instead_of_scrolling() {
    assert_eq!(page_down(&mut plugin_quick_view(true)), 0);
    assert!(page_down(&mut plugin_quick_view(false)) > 0);
}

#[test]
fn files_after_the_cursor_are_preloaded_ahead() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"] {
        std::fs::write(dir.path().join(name), name).unwrap();
    }
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    state.refresh_panel(ActivePanel::Left, false, true);
    let panel = state.panels.side_mut(ActivePanel::Left);
    panel.cursor_index = panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
        .unwrap();
    let names: Vec<String> = state
        .preload_ahead()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["b.txt", "c.txt", "d.txt"]);
}
