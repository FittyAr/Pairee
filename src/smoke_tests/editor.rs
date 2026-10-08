//! Built-in editor (F4): UTF-8 typing, selection with copy / paste, undo /
//! redo, save, save as, CRLF line endings, reopening.

use crate::app::state::{PopupType, Screen};
use crate::test_harness::Harness;

fn editor_lines(h: &Harness) -> Vec<String> {
    match h.state.screens.get(h.state.active_screen_idx) {
        Some(Screen::Editor(ed)) => ed.lines.clone(),
        _ => panic!("editor screen expected\n{}", h.screen_text()),
    }
}

fn in_panels(h: &Harness) -> bool {
    matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Panels)
    )
}

#[test]
fn edit_save_save_as_and_reopen_keep_crlf() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        h.write("work/left/doc.txt", "uno\r\ndos\r\n");
        h.reread().focus("doc.txt").keys("@edit");
        assert_eq!(editor_lines(&h)[..2], ["uno", "dos"], "{keymap}");
        h.assert_screen("uno");

        // UTF-8 typing at the start of the first line.
        h.text("ñ😀 ");
        assert_eq!(editor_lines(&h)[0], "ñ😀 uno");
        h.assert_screen("ñ");

        // Select the rest of the line, copy it, paste it at the end of line 2.
        h.keys("Home Shift+End Ctrl+c Down End Ctrl+v");
        assert_eq!(editor_lines(&h)[1], "dosñ😀 uno");

        // Undo removes the paste, redo brings it back.
        h.keys("Ctrl+z");
        assert_eq!(editor_lines(&h)[1], "dos");
        h.keys("Ctrl+y");
        assert_eq!(editor_lines(&h)[1], "dosñ😀 uno");

        // F2 saves in place, keeping CRLF.
        h.keys("F2");
        assert_eq!(
            String::from_utf8(h.read("work/left/doc.txt")).unwrap(),
            "ñ😀 uno\r\ndosñ😀 uno\r\n",
            "{keymap}"
        );

        // Shift+F2 saves a copy under a new name (relative to the folder).
        h.keys("Shift+F2");
        assert!(matches!(
            h.state.dialogs.top(),
            Some(PopupType::EditorSaveAsPrompt { .. })
        ));
        h.keys("End Backspace*7").text("copia.txt").keys("Enter");
        assert!(h.state.dialogs.is_none(), "{}", h.screen_text());
        assert_eq!(h.read("work/left/copia.txt"), h.read("work/left/doc.txt"));

        // Quit and reopen: the saved text is back.
        h.keys("F10");
        assert!(in_panels(&h), "{keymap}: {}", h.screen_text());
        h.reread().focus("doc.txt").keys("@edit");
        assert_eq!(editor_lines(&h)[..2], ["ñ😀 uno", "dosñ😀 uno"]);
        h.keys("Esc");
        assert!(in_panels(&h));
    }
}

#[test]
fn quitting_with_changes_asks_to_discard() {
    let mut h = Harness::new();
    h.write("work/left/notes.txt", "keep\n");
    h.reread().focus("notes.txt").keys("@edit");
    h.text("lost ");
    h.keys("Esc");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConfirmDiscardEditorChanges)
    ));
    h.keys("Enter");
    assert!(in_panels(&h), "{}", h.screen_text());
    assert_eq!(h.read("work/left/notes.txt"), b"keep\n");
}

#[test]
fn bracketed_paste_inserts_text_in_the_editor_and_in_dialogs() {
    let mut h = Harness::new();
    h.write("work/left/vacio.txt", "");
    h.reread().focus("vacio.txt").keys("@edit");
    h.paste("uno ñ\ndos 😀");
    assert_eq!(editor_lines(&h)[..2], ["uno ñ", "dos 😀"]);
    h.keys("F2 F10");
    assert!(in_panels(&h), "{}", h.screen_text());
    assert_eq!(h.read("work/left/vacio.txt"), "uno ñ\ndos 😀".as_bytes());

    h.keys("@mkdir");
    h.paste("pegada");
    h.keys("Enter");
    assert!(h.left().join("pegada").is_dir());
}
