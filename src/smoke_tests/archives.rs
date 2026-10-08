//! Archives as folders: enter zip / tar.gz, view an entry, copy out with
//! F5, copy into a zip, leave through `..`.

use crate::app::state::{ActivePanel, Screen};
use crate::fs::archive::test_fixtures::{SAMPLE_TREE, write_tar_gz, write_zip};
use crate::test_harness::Harness;

fn in_viewer(h: &Harness) -> bool {
    matches!(
        h.state.screens.get(h.state.active_screen_idx),
        Some(Screen::Viewer(_))
    )
}

/// Enters `archive`, views `sub/b.txt`, copies it out to the right panel
/// and leaves with `..` (cursor back on the archive).
fn browse_view_copy_out(h: &mut Harness, archive: &str) {
    h.reread().focus(archive).keys("Enter");
    let panel = h.state.get_active_panel();
    assert!(panel.source.archive().is_some(), "{archive}");
    h.assert_screen("a.txt");
    h.focus("sub").keys("Enter");
    h.assert_screen("b.txt");

    h.focus("b.txt").keys("@view");
    assert!(in_viewer(h));
    h.wait_for_text("bravo");
    h.keys("Esc");

    h.focus("b.txt").keys("@copy Enter");
    h.wait_until("copy out of the archive", |h| {
        h.right().join("b.txt").is_file()
    });
    assert_eq!(h.read("work/right/b.txt"), b"bravo");
    std::fs::remove_file(h.right().join("b.txt")).unwrap();

    h.focus("..").keys("Enter");
    h.focus("..").keys("Enter");
    assert_eq!(h.state.get_active_panel().current_path, h.left());
    assert!(h.state.get_active_panel().source.is_local());
    assert_eq!(h.cursor_name(), archive);
}

#[test]
fn zip_and_tar_gz_browse_view_copy_out() {
    for keymap in ["norton", "vscode"] {
        let mut h = Harness::builder().keymap(keymap).build();
        write_zip(&h.left().join("bundle.zip"), SAMPLE_TREE);
        write_tar_gz(&h.left().join("bundle.tar.gz"), SAMPLE_TREE);
        browse_view_copy_out(&mut h, "bundle.zip");
        browse_view_copy_out(&mut h, "bundle.tar.gz");
    }
}

#[test]
fn copy_into_a_zip() {
    let mut h = Harness::new();
    write_zip(&h.left().join("bundle.zip"), SAMPLE_TREE);
    h.write("work/right/nuevo.txt", "dentro");
    h.reread().focus("bundle.zip").keys("Enter Tab");
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    h.focus("nuevo.txt").keys("@copy Enter");
    h.keys("Tab");
    h.wait_until("copy into the zip", |h| {
        h.reread();
        h.names().contains(&"nuevo.txt".to_string())
    });
    // The archive on disk holds the new entry.
    let file = std::fs::File::open(h.left().join("bundle.zip")).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut body = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("nuevo.txt").unwrap(), &mut body).unwrap();
    assert_eq!(body, "dentro");
}
