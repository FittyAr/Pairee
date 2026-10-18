//! Archives as folders: enter zip / tar.gz / 7z (and RAR, ISO, WIM... when
//! a full 7-Zip is installed), view an entry, copy out with F5, copy into
//! a zip, leave through `..`; extract whole archives from the menu.

use crate::app::state::{ActivePanel, Screen};
use crate::fs::archive::test_fixtures::{
    SAMPLE_TREE, write_7z, write_tar_gz, write_tree, write_zip,
};
use crate::test_harness::Harness;

fn in_viewer(h: &Harness) -> bool {
    matches!(Some(h.state.screens.active()), Some(Screen::Viewer(_)))
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
        write_7z(&h.left().join("bundle.7z"), SAMPLE_TREE);
        browse_view_copy_out(&mut h, "bundle.zip");
        browse_view_copy_out(&mut h, "bundle.tar.gz");
        // A solid 7z block: entries passed over must not fail its checksum.
        browse_view_copy_out(&mut h, "bundle.7z");
    }
}

/// Packs `SAMPLE_TREE` into `archive` with the installed 7-Zip and the
/// extra `switches`; `false` when no full 7-Zip is installed.
fn pack_with_7z(archive: &std::path::Path, switches: &[&str]) -> bool {
    let Some(tool) = crate::fs::external_tools::find_7z().filter(|t| t.full) else {
        eprintln!("no full 7-Zip installed: skipping {}", archive.display());
        return false;
    };
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), SAMPLE_TREE);
    let status = std::process::Command::new(&tool.path)
        .arg("a")
        .args(switches)
        .arg(archive)
        .arg(src.path().join("*"))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "7z a {}", archive.display());
    true
}

/// Formats without a native reader (WIM here) and 7z codecs the native
/// reader lacks (Deflate64) open, view and copy out through 7-Zip.
#[test]
fn formats_read_through_7_zip_browse_view_copy_out() {
    let mut h = Harness::new();
    if !pack_with_7z(&h.left().join("bundle.wim"), &["-twim"])
        || !pack_with_7z(&h.left().join("deflate64.7z"), &["-t7z", "-m0=Deflate64"])
    {
        return;
    }
    browse_view_copy_out(&mut h, "bundle.wim");
    browse_view_copy_out(&mut h, "deflate64.7z");
}

/// Extract (menu / Shift+F2) unpacks every selected archive into the other
/// panel, and says so when the cursor is on something else.
#[test]
fn extract_unpacks_the_selected_archives() {
    use crate::app::state::popup::InfoMessage;
    use crate::config::localization::t;
    use crate::keybindings::Action;
    let mut h = Harness::new();
    write_zip(&h.left().join("one.zip"), &[("one.txt", b"1")]);
    write_7z(&h.left().join("two.7z"), &[("dir/two.txt", b"2")]);
    h.write("work/left/notes.txt", "plain");
    h.reread()
        .focus("notes.txt")
        .dispatch(Action::ExtractArchive);
    assert_eq!(
        h.state.dialogs.top_as::<InfoMessage>().map(|m| m.0.clone()),
        Some(t("archive_none_selected"))
    );
    h.keys("Esc");
    h.focus("one.zip")
        .keys("Insert")
        .focus("two.7z")
        .keys("Insert");
    h.dispatch(Action::ExtractArchive);
    h.wait_until("both archives extracted", |h| {
        h.exists("work/right/one.txt") && h.exists("work/right/dir/two.txt")
    });
    assert_eq!(h.read("work/right/dir/two.txt"), b"2");
}

/// A job that fails says why, even with the transfer panel minimized.
#[test]
fn a_failed_extraction_is_reported() {
    use crate::app::state::popup::ErrorMessage;
    let mut h = Harness::new();
    h.write("work/left/broken.zip", "not a zip at all");
    h.reread().focus("broken.zip");
    h.dispatch(crate::keybindings::Action::ExtractArchive);
    h.wait_until("the failure is shown", |h| {
        h.state.dialogs.top_as::<ErrorMessage>().is_some()
    });
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

/// The archive commands' List and Test read the archive in the background
/// (a message shows meanwhile) and then show the result.
#[test]
fn archive_list_and_test_run_in_the_background() {
    use crate::app::state::PopupType;
    use crate::app::state::popup::InfoMessage;
    use crate::config::localization::t;
    let mut h = Harness::new();
    write_zip(&h.left().join("bundle.zip"), SAMPLE_TREE);
    h.reread().focus("bundle.zip");
    // Started without settling: the message is up until the job ends.
    h.dispatch(crate::keybindings::Action::ArchiveCommands);
    let archive = h.left().join("bundle.zip");
    crate::app::dialog_loads::read_archive(
        &mut h.state,
        archive,
        crate::app::dialog_loads::ArchiveRead::Test,
    );
    let reading = h
        .state
        .dialogs
        .top_as::<InfoMessage>()
        .map(|InfoMessage(text)| text.clone());
    assert_eq!(reading, Some(t("archive_reading")));
    h.settle();
    match h.state.dialogs.top() {
        Some(PopupType::Info(InfoMessage(text))) => assert_eq!(*text, t("archive_test_ok")),
        other => panic!("{other:?}"),
    }
    h.keys("Esc")
        .dispatch(crate::keybindings::Action::ArchiveCommands);
    h.keys("Enter");
    h.settle();
    assert!(in_viewer(&h), "the listing opens in the viewer");
    h.wait_for_text("b.txt");
}
