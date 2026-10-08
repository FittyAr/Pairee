use super::QUICK_VIEW_DEBOUNCE;
use super::cache::{PreviewCache, PreviewKey};
use super::load::{QuickViewPreview, load_preview, read_text_prefix};
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::config::localization::t;
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
    let p = load_preview(&path, false, 50);
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
    assert_eq!(read_text_prefix(&path, 2).as_deref(), Some("a"));
    std::fs::write(&path, [0x61, 0x00, 0x62, 0x01, 0x02]).unwrap();
    assert_eq!(read_text_prefix(&path, 10), None, "NUL bytes = binary");
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
    assert_eq!(load_preview(&path, false, 1024).content, ["hola", "mundo"]);
    let (latin, _, _) = encoding_rs::WINDOWS_1252.encode(
        "Canción de la niña con su pingüino
",
    );
    std::fs::write(&path, latin).unwrap();
    assert_eq!(
        load_preview(&path, false, 1024).content,
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
            .left
            .entries
            .iter()
            .position(|e| e.name == name)
            .unwrap()
    };

    state.panels.left.cursor_index = idx(&state, "a.txt");
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
    state.panels.left.cursor_index = idx(&state, "b.txt");
    state.update_quick_view_images(false);
    state.panels.left.cursor_index = idx(&state, "a.txt");
    state.update_quick_view_images(false);
    assert_eq!(quick_view_lines(&state), Some(vec!["alpha".to_string()]));
    assert!(state.quick_view.pending.is_none());
}
