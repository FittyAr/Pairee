//! Folder tabs (new, switch, move, close, lock, tab bar) and the session
//! saved on quit and restored by the next run; `--cwd-file`.

use crate::app::state::ActivePanel;
use crate::test_harness::Harness;
use std::path::PathBuf;

fn titles(h: &Harness) -> Vec<String> {
    let side = h.state.panels.active;
    h.state
        .panels
        .tabs(side)
        .tabs()
        .iter()
        .map(|t| t.title())
        .collect()
}

fn active_index(h: &Harness) -> usize {
    h.state.panels.tabs(h.state.panels.active).active_index()
}

/// Left panel with folders `uno` and `dos`, a tab open in each.
fn two_tabs(keymap: &str) -> Harness {
    let mut h = Harness::builder().keymap(keymap).build();
    h.mkdir("work/left/uno/sub");
    h.mkdir("work/left/dos");
    h.reread().focus("uno").keys("Enter @new_tab Backspace");
    h.focus("dos").keys("Enter");
    h
}

#[test]
fn new_switch_move_close_and_tab_bar() {
    for keymap in ["norton", "vscode"] {
        let mut h = two_tabs(keymap);
        assert_eq!(titles(&h), ["uno", "dos"], "{keymap}");
        assert_eq!(active_index(&h), 1);
        h.assert_screen("uno");
        h.assert_screen("dos");

        h.keys("@prev_tab");
        assert_eq!(active_index(&h), 0, "{keymap}");
        assert!(h.state.get_active_panel().current_path.ends_with("uno"));
        h.keys("@next_tab");
        assert_eq!(active_index(&h), 1);
        h.keys("@go_to_tab_1");
        assert_eq!(active_index(&h), 0);

        h.keys("@move_tab_right");
        assert_eq!(titles(&h), ["dos", "uno"], "{keymap}");
        assert_eq!(active_index(&h), 1);

        h.keys("@close_tab");
        assert_eq!(titles(&h), ["dos"], "{keymap}");
        // The last tab cannot be closed.
        h.keys("@close_tab");
        assert_eq!(titles(&h), ["dos"]);
    }
}

#[test]
fn clicking_a_tab_in_the_bar_switches_to_it() {
    let mut h = two_tabs("norton");
    h.render();
    let screen = h.screen_text();
    let (row, line) = screen
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains("uno") && l.contains("dos"))
        .expect("tab bar row");
    let column = line[..line.find("uno").unwrap()].chars().count() as u16;
    h.click(column + 1, row as u16);
    assert_eq!(active_index(&h), 0, "{screen}");
}

#[test]
fn locked_tab_keeps_its_folder() {
    let mut h = Harness::new();
    h.mkdir("work/left/uno/sub");
    h.reread().focus("uno").keys("Enter @toggle_tab_lock");
    let side = h.state.panels.active;
    assert!(h.state.panels.tabs(side).active().lock.is_some());
    // Entering a subfolder opens it in a new tab; the locked one stays.
    h.focus("sub").keys("Enter");
    assert_eq!(titles(&h), ["uno", "sub"]);
    assert!(h.state.get_active_panel().current_path.ends_with("sub"));
    h.keys("@go_to_tab_1");
    assert!(h.state.get_active_panel().current_path.ends_with("uno"));
}

#[test]
fn session_is_restored_by_the_next_run() {
    let mut h = two_tabs("norton");
    h.keys("Tab");
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    let h = h.restart(|b| b.without_cli_folders());
    assert!(h.config_dir().join("session.toml").is_file());
    let left = h.state.panels.tabs(ActivePanel::Left);
    let names: Vec<String> = left.tabs().iter().map(|t| t.title()).collect();
    assert_eq!(names, ["uno", "dos"]);
    assert_eq!(left.active_index(), 1);
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    assert_eq!(
        h.state.panels.side(ActivePanel::Right).current_path,
        h.right()
    );
}

#[test]
fn cwd_file_gets_the_focused_folder() {
    let mut h = two_tabs("norton");
    let exit_dir = h.quit();
    let file = h.root().join("cwd.txt");
    let args = [
        "--cwd-file".to_string(),
        file.to_string_lossy().into_owned(),
    ];
    let launch = crate::launch_args::LaunchArgs::parse(args).unwrap();
    crate::run::report_exit_dir(&launch, exit_dir.as_deref());
    let written = PathBuf::from(std::fs::read_to_string(&file).unwrap());
    assert_eq!(written, h.left().join("dos"));
}
