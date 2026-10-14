//! Views over the panels' files: the branch view, folder jumps (list and
//! `z`), Compare files and the duplicate finder.

use crate::app::state::PopupType;
use crate::app::state::popup::{
    CompareStatus, DupeOptionsPrompt, DuplicatesView, FileCompareView, FolderJump,
};
use crate::config::localization::t;
use crate::test_harness::Harness;
use std::path::Path;

/// The tree view opens at once and gets its folders from a background read.
#[test]
fn tree_view_reads_its_folders_in_the_background() {
    use crate::app::state::popup::TreeView;
    let mut h = Harness::new();
    h.mkdir("work/left/sub/deep");
    h.reread();
    let root = h.left();
    let caller = crate::app::state::types::TreeViewCaller::Panel(h.state.panels.active);
    crate::app::dialog_loads::open_tree(&mut h.state, root, caller, false);
    let tree = h.state.dialogs.top_as::<TreeView>().expect("tree view");
    assert!(tree.loading && tree.nodes.is_empty(), "nothing read yet");
    h.render().assert_screen(&t("tree_view_loading"));
    h.settle();
    let tree = h.state.dialogs.top_as::<TreeView>().expect("tree view");
    assert!(!tree.loading);
    let names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names[1..], ["sub", "deep"]);
}

/// The user menu reads `usermenu.toml` when it opens.
#[test]
fn the_user_menu_reads_its_commands_when_it_opens() {
    use crate::app::state::popup::UserMenu;
    let mut h = Harness::new();
    std::fs::write(
        h.config_dir().join("usermenu.toml"),
        "[commands]\nX = \"echo hi\"\n",
    )
    .unwrap();
    h.keys("@user_menu");
    let menu = h.state.dialogs.top_as::<UserMenu>().expect("user menu");
    assert_eq!(menu.items[0].command.as_deref(), Some("echo hi"));
    h.render().assert_screen("echo hi");
}

/// `a/b/c` with the platform's separator, as the branch view names files.
fn rel(path: &str) -> String {
    Path::new(path)
        .to_string_lossy()
        .replace('/', std::path::MAIN_SEPARATOR_STR)
}

#[test]
fn branch_view_lists_the_subtree_flat_and_works_with_file_operations() {
    let mut h = Harness::new();
    h.write("work/left/top.txt", "1");
    h.write("work/left/a/b/deep.txt", "22");
    h.write("work/left/a/one.rs", "333");
    h.reread().keys("@branch_view");
    h.settle();
    let names = h.names();
    for name in ["top.txt", "a/b/deep.txt", "a/one.rs"] {
        assert!(names.contains(&rel(name)), "{name} in {names:?}");
    }
    assert!(!names.contains(&"a".to_string()), "folders are not listed");
    let title = crate::ui::panel::helpers::build_panel_title(
        h.state.get_active_panel(),
        &h.ctx.config.settings,
    );
    assert!(title.contains(&t("panel_branch_view")), "{title}");

    // F5 copies the file itself (flat) to the other panel.
    h.focus(&rel("a/b/deep.txt")).keys("@copy Enter");
    h.wait_until("copy from the branch view", |h| {
        h.exists("work/right/deep.txt")
    });

    // Renaming keeps the file in its folder.
    h.focus(&rel("a/one.rs")).keys("@rename");
    if let Some(PopupType::RenamePrompt(prompt)) = h.state.dialogs.top() {
        assert_eq!(prompt.original, "one.rs");
    } else {
        panic!("rename prompt expected");
    }
    h.keys("Esc");

    // The action again goes back to the normal listing.
    h.keys("@branch_view");
    h.settle();
    assert!(h.names().contains(&"a".to_string()));
    assert!(!h.names().contains(&rel("a/one.rs")));
}

#[test]
fn leaving_the_folder_ends_the_branch_view() {
    let mut h = Harness::new();
    h.write("work/left/a/x.txt", "1");
    h.reread().keys("@branch_view");
    h.settle();
    assert!(h.state.get_active_panel().branch_view);
    h.keys("@go_parent");
    h.settle();
    assert!(!h.state.get_active_panel().branch_view);
}

#[test]
fn folder_jump_list_and_z_go_to_frequent_folders() {
    let mut h = Harness::new();
    h.mkdir("work/left/projects/media");
    h.mkdir("work/left/other");
    let start = h.left();
    let media = start.join("projects").join("media");
    // Visit the folders (each visit raises its frecency).
    for _ in 0..2 {
        h.text(&format!("cd {}", media.display())).keys("Enter");
        h.text(&format!("cd {}", start.display())).keys("Enter");
    }
    h.text("cd other").keys("Enter");
    h.settle();

    h.keys("@folder_jump");
    assert!(h.state.dialogs.top_as::<FolderJump>().is_some());
    h.text("med").keys("Enter");
    h.settle();
    assert_eq!(h.state.get_active_panel().current_path, media);

    h.text(&format!("cd {}", start.display())).keys("Enter");
    h.text("z proj med").keys("Enter");
    h.settle();
    assert_eq!(h.state.get_active_panel().current_path, media);

    h.text("z nothing-like-this").keys("Enter");
    assert!(matches!(h.state.dialogs.top(), Some(PopupType::Info(_))));
}

#[test]
fn folder_jump_list_hides_folders_that_are_gone() {
    let mut h = Harness::new();
    h.mkdir("work/left/kept");
    h.mkdir("work/left/doomed");
    let start = h.left();
    for name in ["kept", "doomed"] {
        h.text(&format!("cd {}", start.join(name).display()))
            .keys("Enter");
        h.text(&format!("cd {}", start.display())).keys("Enter");
    }
    std::fs::remove_dir(start.join("doomed")).unwrap();

    h.keys("@folder_jump");

    let jump = h.state.dialogs.top_as::<FolderJump>().unwrap();
    assert!(jump.items.contains(&start.join("kept")), "{:?}", jump.items);
    assert!(
        !jump.items.contains(&start.join("doomed")),
        "{:?}",
        jump.items
    );
    assert!(
        h.state
            .frecency
            .ranked("doomed", crate::config::frecency::now())
            .is_empty(),
        "forgotten, not only hidden"
    );
}

#[test]
fn compare_files_shows_a_diff_or_hashes() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "one\ntwo\nthree\n");
    h.write("work/right/a.txt", "one\n2\nthree\n");
    h.reread()
        .focus("a.txt")
        .keys("Tab")
        .focus("a.txt")
        .keys("Tab");
    h.keys("@compare_files");
    h.settle();
    let view = h.state.dialogs.top_as::<FileCompareView>().expect("view");
    assert_eq!(view.status, CompareStatus::Text);
    assert_eq!(view.viewer.doc.stats(), (1, 1));
    h.assert_screen("-two");
    h.keys("Tab");
    h.assert_screen("│");
    h.keys("Esc");
    assert!(h.state.dialogs.is_empty());

    h.write("work/left/b.bin", [0u8, 1, 2]);
    h.write("work/right/b.bin", [0u8, 1, 2]);
    h.reread()
        .focus("b.bin")
        .keys("Tab")
        .focus("b.bin")
        .keys("Tab");
    h.keys("@compare_files");
    h.settle();
    let view = h.state.dialogs.top_as::<FileCompareView>().expect("view");
    assert!(matches!(&view.status, CompareStatus::Binary(b) if b.identical));
    h.assert_screen(&t("compare_files_binary_identical"));
}

#[test]
fn duplicate_finder_marks_and_deletes_copies() {
    let mut h = Harness::new();
    for name in ["a.txt", "b.txt", "c.txt"] {
        h.write(&format!("work/left/{name}"), "same contents");
    }
    h.write("work/left/unique.txt", "something else");
    h.reread().keys("@find_duplicates");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::DupeOptionsPrompt(_))
    ));
    // The follow-links checkbox toggles (on and off again).
    h.keys("Down Down Space");
    let prompt = h.state.dialogs.top_as::<DupeOptionsPrompt>().unwrap();
    assert!(prompt.follow_links);
    h.assert_screen(&format!("[x] {}", t("dupes_follow_links")));
    h.keys("Space Up Up");
    h.keys("Enter");
    h.settle();
    assert!(h.state.dialogs.top_as::<DuplicatesView>().is_some());
    assert_eq!(h.state.dupes.file_count(), 3);
    h.assert_screen("c.txt");
    h.assert_no_screen("unique.txt");

    // Mark all but the first copy, then delete them through the usual
    // confirmation.
    h.keys("f Delete");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConfirmDelete(_))
    ));
    h.keys("Enter");
    h.wait_until("delete the marked copies", |h| {
        !h.exists("work/left/b.txt") && !h.exists("work/left/c.txt")
    });
    h.settle();
    assert!(h.exists("work/left/a.txt"));
    assert!(h.state.dialogs.top_as::<DuplicatesView>().is_some());
    assert_eq!(h.state.dupes.file_count(), 0);
    h.keys("Esc");
    assert!(h.state.dialogs.is_empty());
}

#[test]
fn duplicate_finder_goes_to_a_file() {
    let mut h = Harness::new();
    h.write("work/left/x/one.bin", "dup");
    h.write("work/left/y/two.bin", "dup");
    h.reread().keys("@find_duplicates Enter");
    h.settle();
    h.keys("Down Enter");
    h.settle();
    assert!(h.state.dialogs.is_empty());
    assert_eq!(h.state.get_active_panel().current_path, h.left().join("y"));
    assert_eq!(h.cursor_name(), "two.bin");
}
