//! The mouse router: dialogs on top and clicks outside them.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

fn top(h: &Harness) -> Option<&PopupType> {
    h.state.dialogs.top()
}

#[test]
fn clicking_outside_a_transient_popup_closes_it() {
    let mut h = Harness::new();
    h.keys("@about");
    assert!(matches!(top(&h), Some(PopupType::About { .. })));
    h.click(0, 1);
    assert!(top(&h).is_none(), "{}", h.screen_text());
}

#[test]
fn clicking_inside_a_popup_keeps_it_open() {
    let mut h = Harness::new();
    h.keys("@about");
    h.click(50, 15);
    assert!(matches!(top(&h), Some(PopupType::About { .. })));
}

#[test]
fn a_dialog_with_typed_input_survives_a_stray_click() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    assert!(matches!(top(&h), Some(PopupType::MkDirPrompt { .. })));
    h.click(0, 1);
    assert!(matches!(top(&h), Some(PopupType::MkDirPrompt { .. })));
}

use crate::app::state::{ActivePanel, SortField};
use crate::config::settings::RightClick;
use crate::ui::hit::HitTarget;
use crossterm::event::{KeyModifiers, MouseButton};

/// Left panel with `sub/`, three files and `n` extra numbered files.
fn panels(extra: usize) -> Harness {
    let h = Harness::new();
    h.mkdir("work/left/sub");
    for name in ["alpha.txt", "beta.txt", "gamma.txt"] {
        h.write(&format!("work/left/{name}"), name);
    }
    for i in 0..extra {
        h.write(&format!("work/left/n{i:02}.dat"), "x");
    }
    h.write("work/right/zeta.txt", "z");
    let mut h = h;
    h.reread();
    h
}

fn tagged(h: &Harness) -> Vec<String> {
    let panel = h.state.get_active_panel();
    let mut names: Vec<String> = panel
        .entries
        .iter()
        .filter(|e| panel.selected_paths.contains(&e.path))
        .map(|e| e.name.clone())
        .collect();
    names.sort();
    names
}

#[test]
fn a_click_focuses_the_panel_and_moves_its_cursor() {
    let mut h = panels(0);
    let (x, y) = h.locate("zeta.txt");
    h.click(x, y);
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    assert_eq!(h.cursor_name(), "zeta.txt");
    let (x, y) = h.locate("beta.txt");
    h.click(x, y);
    assert_eq!(h.state.panels.active, ActivePanel::Left);
    assert_eq!(h.cursor_name(), "beta.txt");
}

#[test]
fn a_double_click_enters_a_folder() {
    let mut h = panels(0);
    let (x, y) = h.locate("sub");
    h.double_click(x, y);
    assert_eq!(
        h.state.get_active_panel().current_path,
        h.left().join("sub")
    );
}

#[test]
fn ctrl_and_shift_clicks_tag_entries() {
    let mut h = panels(0);
    let (x, y) = h.locate("alpha.txt");
    h.press(MouseButton::Left, x, y, KeyModifiers::CONTROL);
    assert_eq!(tagged(&h), ["alpha.txt"]);
    h.press(MouseButton::Left, x, y, KeyModifiers::CONTROL);
    assert!(tagged(&h).is_empty());
    h.click(x, y);
    let (x, y) = h.locate("gamma.txt");
    h.press(MouseButton::Left, x, y, KeyModifiers::SHIFT);
    assert_eq!(tagged(&h), ["alpha.txt", "beta.txt", "gamma.txt"]);
}

#[test]
fn right_click_tags_and_right_drag_tags_a_range() {
    let mut h = panels(0);
    let (x, y) = h.locate("alpha.txt");
    h.right_click(x, y);
    assert_eq!(tagged(&h), ["alpha.txt"]);
    h.right_click(x, y);
    assert!(tagged(&h).is_empty());
    let (_, to) = h.locate("gamma.txt");
    h.drag(MouseButton::Right, (x, y), (x, to), KeyModifiers::NONE);
    assert_eq!(tagged(&h), ["alpha.txt", "beta.txt", "gamma.txt"]);
    assert_eq!(h.cursor_name(), "gamma.txt");
}

#[test]
fn right_click_can_open_the_context_menu() {
    let mut h = Harness::builder()
        .settings(|s| s.mouse_right_click = RightClick::Menu)
        .build();
    h.write("work/left/alpha.txt", "a");
    h.reread();
    let (x, y) = h.locate("alpha.txt");
    h.right_click(x, y);
    assert_eq!(h.cursor_name(), "alpha.txt");
    assert!(matches!(top(&h), Some(PopupType::ContextMenu { .. })));
    // A click elsewhere closes it again.
    h.click(0, 1);
    assert!(top(&h).is_none());
}

#[test]
fn the_wheel_moves_the_cursor_of_the_panel_under_it() {
    for (extra, scrollbar) in [(0, false), (40, false), (40, true)] {
        let mut h = Harness::builder()
            .settings(|s| s.show_scrollbar = scrollbar)
            .build();
        let mut h = {
            for i in 0..extra {
                h.write(&format!("work/left/n{i:02}.dat"), "x");
            }
            h.write("work/left/a.txt", "a");
            h.write("work/left/b.txt", "b");
            h.write("work/left/c.txt", "c");
            h.write("work/left/d.txt", "d");
            h.reread();
            h
        };
        let (x, y) = h.locate("a.txt");
        h.wheel(x, y, true);
        let cursor = h.state.get_active_panel().cursor_index;
        assert!(
            cursor >= 3,
            "extra={extra} scrollbar={scrollbar}: cursor {cursor}"
        );
        h.wheel(x, y, false);
        h.wheel(x, y, false);
        assert!(
            h.state.get_active_panel().cursor_index < cursor,
            "extra={extra} scrollbar={scrollbar}"
        );
    }
}

#[test]
fn column_titles_sort_and_a_second_click_reverses() {
    let mut h = panels(0);
    let size = |t: &HitTarget| {
        matches!(
            t,
            HitTarget::PanelHeader {
                side: ActivePanel::Left,
                sort: Some(SortField::Size)
            }
        )
    };
    let (x, y) = h.hit_cell(size);
    h.click(x, y);
    assert_eq!(h.state.get_active_panel().sort_field, SortField::Size);
    let reverse = h.state.get_active_panel().sort_reverse;
    h.click(x, y);
    assert_ne!(h.state.get_active_panel().sort_reverse, reverse);
}

#[test]
fn the_folder_title_opens_the_folder_history() {
    let mut h = panels(0);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PanelTitle(ActivePanel::Right));
    h.click(x + 2, y);
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    assert!(matches!(
        top(&h),
        Some(PopupType::FoldersHistoryList { .. })
    ));
}

#[test]
fn pointer_motion_alone_changes_nothing() {
    let mut h = panels(0);
    let (x, y) = h.locate("gamma.txt");
    let before = h.cursor_name();
    h.hover(x, y);
    assert_eq!(h.cursor_name(), before);
    assert!(top(&h).is_none());
}
