//! Panel-source follow-ups: default keys for folder sizes and disk usage,
//! F4 on an archive entry (local copy written back on save, with the
//! "changed meanwhile" check), copies into a zip that respect the conflict
//! setting, tar.bz2 / tar.xz and archives inside archives. SFTP needs a
//! server: those paths are unit tested (`fs::ssh`, `app::editor::remote`).

use crate::app::state::{PopupType, Screen};
use crate::fs::archive::test_fixtures::{
    SAMPLE_TREE, write_tar_bz2, write_tar_gz, write_tar_xz, write_zip,
};
use crate::test_harness::Harness;
use std::io::Read;
use std::path::Path;

fn zip_entry(archive: &Path, name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).unwrap()).unwrap();
    let mut body = String::new();
    zip.by_name(name)
        .unwrap()
        .read_to_string(&mut body)
        .unwrap();
    body
}

fn in_editor(h: &Harness) -> bool {
    matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Editor(_))
    )
}

#[test]
fn alt_s_and_alt_d_measure_and_open_disk_usage() {
    for keymap in ["norton", "neovim", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        h.write("work/left/media/clip.bin", vec![0u8; 3000]);
        h.reread().keys("Alt+s");
        h.wait_until("folder size", |h| {
            let media = h.left().join("media");
            h.state.get_active_panel().dir_sizes.get(&media).is_some()
        });
        h.keys("Alt+d");
        assert!(
            matches!(h.state.dialogs.top(), Some(PopupType::DiskUsage)),
            "{keymap}"
        );
        h.assert_screen("media/");
    }
}

#[test]
fn f4_edits_a_zip_entry_and_saves_it_back() {
    let mut h = Harness::new();
    let archive = h.left().join("bundle.zip");
    write_zip(&archive, SAMPLE_TREE);
    h.reread().focus("bundle.zip").keys("Enter");
    h.focus("a.txt").keys("@edit");
    h.wait_until("editor on the copy", |h| in_editor(h));
    h.assert_screen("bundle.zip");
    h.text("new ").keys("F2");
    h.wait_until("written back", |h| {
        zip_entry(&h.left().join("bundle.zip"), "a.txt") == "new alpha"
    });

    // Changed by someone else meanwhile: saving asks first.
    write_zip(&archive, &[("a.txt", b"theirs")]);
    h.text("x").keys("F2");
    h.wait_until("conflict question", |h| {
        matches!(
            h.state.dialogs.top(),
            Some(PopupType::EditorConfirmOverwrite { .. })
        )
    });
    assert_eq!(zip_entry(&archive, "a.txt"), "theirs");
    h.keys("Enter");
    h.wait_until("overwritten", |h| {
        zip_entry(&h.left().join("bundle.zip"), "a.txt") == "new xalpha"
    });
}

#[test]
fn copying_into_a_zip_asks_about_existing_entries() {
    let mut h = Harness::new();
    let archive = h.left().join("bundle.zip");
    write_zip(&archive, &[("a.txt", b"old"), ("keep.txt", b"k")]);
    h.write("work/right/a.txt", "new");
    h.write("work/right/b.txt", "b");
    h.reread().focus("bundle.zip").keys("Enter Tab");
    h.focus("a.txt")
        .keys("Insert")
        .focus("b.txt")
        .keys("Insert");
    h.keys("@copy Enter");
    h.wait_until("conflict dialog", |h| {
        h.state
            .transfer
            .as_ref()
            .is_some_and(|t| t.active_conflict_info.is_some())
    });
    h.assert_screen("Conflict");
    h.keys("s");
    h.wait_until("b.txt copied in", |h| {
        let file = std::fs::File::open(h.left().join("bundle.zip")).unwrap();
        zip::ZipArchive::new(file).unwrap().by_name("b.txt").is_ok()
    });
    assert_eq!(
        zip_entry(&archive, "a.txt"),
        "old",
        "existing entry skipped"
    );
}

#[test]
fn bzip2_xz_and_nested_archives_open_as_folders() {
    let mut h = Harness::new();
    write_tar_bz2(&h.left().join("t.tar.bz2"), SAMPLE_TREE);
    write_tar_xz(&h.left().join("t.txz"), SAMPLE_TREE);
    for name in ["t.tar.bz2", "t.txz"] {
        h.reread().focus(name).keys("Enter");
        h.assert_screen("a.txt");
        h.focus("..").keys("Enter");
        assert_eq!(h.cursor_name(), name);
    }

    let inner = h.left().join("inner.tar.gz");
    write_tar_gz(&inner, SAMPLE_TREE);
    let bytes = std::fs::read(&inner).unwrap();
    std::fs::remove_file(&inner).unwrap();
    write_zip(&h.left().join("outer.zip"), &[("inner.tar.gz", &bytes)]);
    h.reread().focus("outer.zip").keys("Enter");
    h.focus("inner.tar.gz").keys("Enter");
    h.wait_until("nested listing", |h| h.names().contains(&"sub".to_string()));
    h.focus("a.txt").keys("@view");
    h.wait_for_text("alpha");
    h.keys("Esc");
    h.focus("..").keys("Enter");
    assert_eq!(h.cursor_name(), "inner.tar.gz");
    assert_eq!(
        h.state.get_active_panel().current_path,
        h.left().join("outer.zip")
    );
}
