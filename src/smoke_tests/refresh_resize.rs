//! Panel auto-refresh after an external change, and every screen painted
//! at tiny terminal sizes without panicking.

use crate::app::state::{PopupType, Screen};
use crate::fs::archive::test_fixtures::{SAMPLE_TREE, write_zip};
use crate::test_harness::Harness;

#[test]
fn panel_follows_files_created_outside() {
    let mut h = Harness::new();
    h.reread();
    assert!(!h.names().contains(&"externo.txt".to_string()));
    h.write("work/left/externo.txt", "x");
    // Watch events are debounced (network drives are polled): be tolerant.
    h.wait_until("auto-refresh", |h| {
        h.names().contains(&"externo.txt".to_string())
    });
    h.assert_screen("externo.txt");
    std::fs::remove_file(h.left().join("externo.txt")).unwrap();
    h.wait_until("auto-refresh after delete", |h| {
        !h.names().contains(&"externo.txt".to_string())
    });
}

/// Sizes that squeeze every widget: no room at all, one row, one column,
/// very wide and short, very narrow and tall.
const SIZES: [(u16, u16); 8] = [
    (1, 1),
    (2, 2),
    (10, 5),
    (20, 6),
    (40, 12),
    (200, 3),
    (3, 60),
    (100, 30),
];

fn survive_tiny_sizes(h: &mut Harness, what: &str) {
    for (width, height) in SIZES {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            h.resize(width, height);
        }));
        assert!(result.is_ok(), "{what} panicked at {width}x{height}");
    }
}

fn in_panels(h: &Harness) -> bool {
    matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Panels)
    )
}

/// A sandbox with something to show in every view.
fn busy_sandbox() -> Harness {
    let mut h = Harness::new();
    h.write("work/left/texto.txt", "línea uno\nlínea dos 😀\n");
    h.write("work/left/fotos/a.jpg", "a");
    write_zip(&h.left().join("bundle.zip"), SAMPLE_TREE);
    h.write("work/right/otro.txt", "otro");
    h.reread();
    h
}

#[test]
fn panels_tabs_and_dialogs_at_tiny_sizes() {
    let mut h = busy_sandbox();
    survive_tiny_sizes(&mut h, "panels");
    h.focus("fotos").keys("@new_tab");
    survive_tiny_sizes(&mut h, "tab bar");
    for (script, what) in [
        ("@copy", "transfer dialog"),
        ("@mkdir", "mkdir dialog"),
        ("@multi_rename_tool", "multi-rename"),
        ("@hotlist", "hotlist"),
        ("@folder_shortcuts_config", "folder shortcuts"),
        ("@sync_dirs", "sync dialog"),
        ("@disk_usage", "disk usage"),
        ("@quick_filter", "quick filter"),
        ("@delete", "delete confirmation"),
        ("@menu", "menu"),
        ("@help", "help"),
    ] {
        h.focus("texto.txt").keys(script);
        assert!(h.state.dialogs.is_some(), "{what} opened");
        survive_tiny_sizes(&mut h, what);
        h.state.dialogs.clear();
        h.resize(100, 30);
    }
}

#[test]
fn editor_viewer_and_archive_at_tiny_sizes() {
    let mut h = busy_sandbox();
    h.focus("texto.txt").keys("@edit Shift+Down");
    survive_tiny_sizes(&mut h, "editor");
    h.keys("F7");
    survive_tiny_sizes(&mut h, "editor search");
    // Esc closes the search, then clears the selection, then the editor.
    h.keys("Esc Esc Esc");
    assert!(in_panels(&h), "{}", h.screen_text());
    h.resize(100, 30).focus("texto.txt").keys("@view");
    assert!(matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Viewer(_))
    ));
    survive_tiny_sizes(&mut h, "viewer");
    h.keys("F4");
    survive_tiny_sizes(&mut h, "hex viewer");
    h.keys("F8");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ViewerEncoding { .. })
    ));
    survive_tiny_sizes(&mut h, "encoding selector");
    h.keys("Esc Esc");
    assert!(in_panels(&h), "{}", h.screen_text());
    h.resize(100, 30).focus("bundle.zip").keys("Enter");
    survive_tiny_sizes(&mut h, "archive panel");
}

#[test]
fn git_panel_at_tiny_sizes() {
    let mut h = busy_sandbox();
    git2::Repository::init(h.left()).unwrap();
    h.keys("@open_git_panel");
    h.wait_until("git panel", |h| {
        matches!(h.state.dialogs.top(), Some(PopupType::GitPanel(p)) if !p.status_entries.is_empty())
    });
    for tab in 0..5 {
        survive_tiny_sizes(&mut h, &format!("git panel tab {tab}"));
        h.resize(100, 30).keys("Tab");
    }
}
