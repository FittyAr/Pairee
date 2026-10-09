//! Pairee's file clipboard: yank / cut in one folder, paste in another.

use crate::keybindings::Action;
use crate::test_harness::Harness;

fn names_in(h: &Harness, dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(h.root().join(dir))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn yank_then_paste_copies_and_keeps_the_clipboard() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread().focus("a.txt");
    h.dispatch(Action::Yank);
    h.dispatch(Action::ChangePanel);
    h.dispatch(Action::Paste);
    h.wait_until("copy", |h| h.exists("work/right/a.txt"));
    assert!(h.exists("work/left/a.txt"));
    assert!(
        h.state.file_clipboard.is_some(),
        "a copy can be pasted again"
    );
}

#[test]
fn cut_then_paste_moves_and_empties_the_clipboard() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread().focus("a.txt");
    h.dispatch(Action::Cut);
    h.assert_screen("[1 cut]");
    h.dispatch(Action::ChangePanel);
    h.dispatch(Action::Paste);
    h.wait_until("move", |h| !h.exists("work/left/a.txt"));
    assert!(h.exists("work/right/a.txt"));
    assert!(h.state.file_clipboard.is_none());
}

#[test]
fn pasting_a_copy_in_its_own_folder_keeps_both() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread().focus("a.txt");
    h.dispatch(Action::Yank);
    h.dispatch(Action::Paste);
    h.wait_until("duplicate", |h| names_in(h, "work/left").len() == 2);
    assert!(names_in(&h, "work/left").contains(&"a.txt".to_string()));
}

#[test]
fn cut_pasted_in_place_and_clear_do_nothing() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread().focus("a.txt");
    h.dispatch(Action::Cut);
    h.dispatch(Action::Paste);
    h.settle();
    assert_eq!(names_in(&h, "work/left"), ["a.txt"]);
    h.dispatch(Action::ClearClipboard);
    assert!(h.state.file_clipboard.is_none());
    h.dispatch(Action::Paste);
    h.settle();
    assert_eq!(names_in(&h, "work/left"), ["a.txt"]);
}
