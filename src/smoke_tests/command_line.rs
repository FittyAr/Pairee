//! The command line: `cd` with quotes, `~`, `-` and relative paths, editing
//! with the cursor keys and Ctrl+W, Tab completion and a long prompt that
//! does not hide what is typed.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

fn cd(h: &mut Harness, line: &str) {
    h.text(line).keys("Enter");
}

#[test]
fn cd_takes_quotes_previous_and_relative_folders() {
    let mut h = Harness::new();
    h.mkdir("work/left/My Docs/inner");
    let start = h.left();

    // Drag & drop and "insert path" quote names with spaces.
    cd(&mut h, "cd \"My Docs\"");
    assert_eq!(
        h.state.get_active_panel().current_path,
        start.join("My Docs")
    );
    cd(&mut h, "cd inner");
    assert_eq!(
        h.state.get_active_panel().current_path,
        start.join("My Docs/inner")
    );
    cd(&mut h, "cd ../..");
    assert_eq!(h.state.get_active_panel().current_path, start);
    cd(&mut h, "cd -");
    assert_eq!(
        h.state.get_active_panel().current_path,
        start.join("My Docs/inner")
    );
    assert!(
        h.state.history.folders.iter().any(|p| p.ends_with("inner")),
        "cd records the folder history"
    );

    cd(&mut h, "cd nowhere");
    assert!(matches!(h.state.dialogs.top(), Some(PopupType::Error(_))));
    h.keys("Esc");
    assert_eq!(
        h.state.get_active_panel().current_path,
        start.join("My Docs/inner")
    );
}

#[test]
fn cd_tilde_goes_home() {
    let mut h = Harness::new();
    cd(&mut h, "cd ~");
    let home = crate::app::session::restore::home_dir();
    assert_eq!(h.state.get_active_panel().current_path, home);
}

#[test]
fn line_is_edited_with_the_cursor_keys() {
    let mut h = Harness::new();
    h.text("echo wrld").keys("Left Left Left");
    h.text("o");
    assert_eq!(h.state.cli.text(), "echo world");
    h.keys("Home Delete").text("E");
    assert_eq!(h.state.cli.text(), "Echo world");
    h.keys("End Ctrl+w");
    assert_eq!(h.state.cli.text(), "Echo ");
    h.text("ñandú").keys("Backspace");
    assert_eq!(
        h.state.cli.text(),
        "Echo ñand",
        "Backspace removes one grapheme"
    );
    h.keys("Esc");
    assert!(h.state.cli.text().is_empty());
}

#[test]
fn tab_completes_names_and_cycles() {
    let mut h = Harness::new();
    h.mkdir("work/left/alpha dir");
    h.write("work/left/alpha.txt", "x");
    h.reread();
    h.text("cd al").keys("Tab");
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(h.state.cli.text(), format!("cd \"alpha dir{sep}\""));
    h.keys("Tab");
    assert_eq!(h.state.cli.text(), "cd alpha.txt");
    h.keys("Tab Enter");
    assert_eq!(
        h.state.get_active_panel().current_path,
        h.left().join("alpha dir")
    );
}

#[test]
fn long_prompt_keeps_the_typed_text_visible() {
    let mut h = Harness::builder().size(40, 12).build();
    let deep = "work/left/a_very_long_folder_name_to_fill_the_prompt/another_long_one";
    h.mkdir(deep);
    let line = format!("cd {}", h.root().join(deep).display());
    cd(&mut h, &line);
    h.text("echo typed-text-stays-visible");
    h.assert_screen("visible");
}
