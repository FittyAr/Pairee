//! Internal viewer (F3): legacy encodings, search, hex view, encoding
//! selector and a large file.

use crate::app::state::{PopupType, Screen};
use crate::test_harness::Harness;
use crate::ui::viewer::{ViewerMode, ViewerState};

fn viewer(h: &Harness) -> &ViewerState {
    match h.state.screens.get(h.state.active_screen_idx) {
        Some(Screen::Viewer(vw)) => vw,
        _ => panic!("viewer screen expected\n{}", h.screen_text()),
    }
}

/// Opens `name` (in the left panel) in the viewer and waits for the index.
fn open(h: &mut Harness, name: &str) {
    h.reread().focus(name).keys("@view");
    h.wait_until("viewer index", |h| viewer(h).doc.index_progress().is_none());
}

#[test]
fn latin1_text_search_hex_and_encoding_selector() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        let mut body = b"caf\xe9 se\xf1or\n".to_vec();
        for n in 0..200 {
            body.extend_from_slice(format!("relleno {n}\n").as_bytes());
        }
        body.extend_from_slice(b"aguja final\n");
        h.write("work/left/latin1.txt", &body);
        open(&mut h, "latin1.txt");
        h.assert_screen("café señor");

        // F7 searches from the current line and scrolls to the hit.
        h.keys("F7");
        assert!(matches!(
            h.state.dialogs.top(),
            Some(PopupType::ViewerSearchPrompt(_))
        ));
        h.text("aguja").keys("Enter");
        assert_eq!(viewer(&h).scroll, 201, "{keymap}");
        h.assert_screen("aguja final");

        // F4 toggles the hex dump and back.
        // The find dialog stays open for the next Enter; Esc closes it.
        h.keys("Esc Home F4");
        assert_eq!(viewer(&h).mode, ViewerMode::Hex);
        h.assert_screen("63 61 66 E9");
        h.keys("F4");
        assert_eq!(viewer(&h).mode, ViewerMode::Text);

        // F8: pick UTF-8 explicitly; é is no longer valid.
        h.keys("F8");
        assert!(matches!(
            h.state.dialogs.top(),
            Some(PopupType::ViewerEncoding { .. })
        ));
        h.keys("Home Enter");
        assert_eq!(viewer(&h).doc.encoding(), encoding_rs::UTF_8);
        h.wait_until("reindex", |h| viewer(h).doc.index_progress().is_none());
        h.assert_screen("caf\u{FFFD}");
        h.keys("Esc");
        assert!(matches!(
            h.state.screens.get(h.state.active_screen_idx),
            Some(Screen::Panels)
        ));
    }
}

#[test]
fn large_sparse_file_opens_and_scrolls() {
    let mut h = Harness::new();
    let path = h.write("work/left/big.bin", b"head\n");
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    // Sparse (or lazily zero-filled) on the usual filesystems: cheap.
    file.set_len(300 * 1024 * 1024).unwrap();
    drop(file);
    h.reread().focus("big.bin").keys("@view");
    assert!(!viewer(&h).loading);
    h.render();
    h.keys("PageDown End F4 End Home F4");
    h.render();
    h.keys("Esc");
    assert!(matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Panels)
    ));
}
