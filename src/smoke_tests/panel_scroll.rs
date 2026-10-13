//! Stable panel scrolling (as in Far and MC): the view only moves when the
//! cursor leaves it, so the rows under the mouse stay put between the two
//! clicks of a double click.

use crate::app::state::Screen;
use crate::test_harness::Harness;

/// A harness whose left panel lists `count` files `f00.txt`, `f01.txt`...
fn many_files(count: usize) -> Harness {
    let mut h = Harness::new();
    for i in 0..count {
        h.write(&format!("work/left/f{i:02}.txt"), "x");
    }
    h.reread();
    h
}

fn first_shown(h: &mut Harness) -> usize {
    h.render();
    h.state.get_active_panel().scroll_top.get()
}

#[test]
fn moving_inside_the_view_does_not_scroll_it() {
    let mut h = many_files(80);
    h.keys("Home");
    assert_eq!(first_shown(&mut h), 0);
    h.keys("Down*15");
    assert_eq!(first_shown(&mut h), 0, "the cursor is still in view");
    h.keys("End");
    let bottom = first_shown(&mut h);
    assert!(bottom > 0);
    h.keys("Up*5");
    assert_eq!(first_shown(&mut h), bottom, "moving up inside the view");
    h.keys("Home");
    assert_eq!(first_shown(&mut h), 0);
}

#[test]
fn double_click_low_in_a_long_list_opens_the_clicked_file() {
    let mut h = many_files(80);
    h.keys("Home");
    // A row in the lower half: a centered view moved it between the clicks.
    let (x, y) = h.locate("f18.txt");
    h.double_click(x, y);
    match h.state.screens.active() {
        Screen::Viewer(viewer) => assert!(viewer.path.ends_with("f18.txt")),
        other => panic!("expected the viewer on f18.txt, got {other:?}"),
    }
}

#[test]
fn double_click_after_scrolling_hits_the_same_entry() {
    let mut h = many_files(80);
    h.keys("End Up*3");
    let (x, y) = h.locate("f70.txt");
    h.click(x, y);
    assert_eq!(h.cursor_name(), "f70.txt");
    assert_eq!(h.locate("f70.txt"), (x, y), "the row did not move");
    h.double_click(x, y);
    match h.state.screens.active() {
        Screen::Viewer(viewer) => assert!(viewer.path.ends_with("f70.txt")),
        other => panic!("expected the viewer on f70.txt, got {other:?}"),
    }
}
