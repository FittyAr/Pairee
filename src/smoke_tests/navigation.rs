//! Panel navigation: folders in and out, hidden files, symlinked folders,
//! quick filter.

use crate::test_harness::Harness;

fn sandbox_with_tree(keymap: &str) -> Harness {
    let h = Harness::builder().keymap(keymap).build();
    h.write("work/left/docs/readme.txt", "hello");
    h.write("work/left/docs/inner/deep.txt", "deep");
    h.write("work/left/alpha.txt", "a");
    h.write("work/left/beta.log", "b");
    h.write("work/left/.secret", "s");
    h
}

#[test]
fn enter_and_leave_folders_in_both_keymaps() {
    for keymap in ["norton", "vscode"] {
        let mut h = sandbox_with_tree(keymap);
        h.reread();
        h.assert_screen("docs");
        h.focus("docs").keys("Enter");
        assert_eq!(
            h.state.get_active_panel().current_path,
            h.left().join("docs")
        );
        h.assert_screen("readme.txt");
        h.focus("inner").keys("Enter");
        h.assert_screen("deep.txt");
        // Backspace goes up and lands on the folder we left.
        h.keys("Backspace");
        assert_eq!(h.cursor_name(), "inner", "{keymap}");
        // `..` + Enter goes up too.
        h.focus("..").keys("Enter");
        assert_eq!(h.state.get_active_panel().current_path, h.left());
        assert_eq!(h.cursor_name(), "docs", "{keymap}");
    }
}

#[test]
fn hidden_files_toggle() {
    for keymap in ["norton", "vscode"] {
        let mut h = sandbox_with_tree(keymap);
        h.reread();
        assert!(!h.names().contains(&".secret".to_string()), "{keymap}");
        h.keys("@toggle_hidden");
        assert!(h.ctx.config.settings.show_hidden, "{keymap}");
        assert!(h.names().contains(&".secret".to_string()), "{keymap}");
        h.assert_screen(".secret");
        h.keys("@toggle_hidden");
        assert!(!h.names().contains(&".secret".to_string()), "{keymap}");
    }
}

#[cfg(unix)]
#[test]
fn symlinked_folder_opens_as_a_folder() {
    let mut h = sandbox_with_tree("norton");
    std::os::unix::fs::symlink(h.left().join("docs"), h.left().join("link")).unwrap();
    h.reread();
    h.focus("link").keys("Enter");
    assert_eq!(
        h.state.get_active_panel().current_path,
        h.left().join("link")
    );
    h.assert_screen("readme.txt");
    assert!(h.state.dialogs.is_none());
}

/// Entries without `..`, in panel order.
fn listed(h: &Harness) -> Vec<String> {
    h.names().into_iter().filter(|n| n != "..").collect()
}

#[test]
fn quick_filter_brings_matches_first_and_esc_restores() {
    for keymap in ["norton", "vscode"] {
        let mut h = sandbox_with_tree(keymap);
        h.reread();
        let before = listed(&h);
        // Matches (substring, any case) move to the top of the list.
        h.keys("@quick_filter").text("BET");
        assert_eq!(listed(&h)[0], "beta.log", "{keymap}");
        h.assert_screen("BET");
        h.keys("Esc");
        assert!(h.state.dialogs.is_none());
        assert_eq!(listed(&h), before, "{keymap}: Esc restores the order");
        // Enter keeps the filter.
        h.keys("@quick_filter").text("alp").keys("Enter");
        assert!(h.state.dialogs.is_none());
        assert_eq!(listed(&h)[0], "alpha.txt", "{keymap}");
        assert_eq!(
            h.state.get_active_panel().quick_filter_mask.as_deref(),
            Some("alp")
        );
    }
}
