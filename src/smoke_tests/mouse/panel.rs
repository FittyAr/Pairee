//! Clicks, double clicks, tagging and the wheel on the panels, the border
//! between them and the screens behind.

use super::{panels, top};
use crate::app::state::{ActivePanel, PopupType, SortField};
use crate::config::settings::RightClick;
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;
use crossterm::event::{KeyModifiers, MouseButton};

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
    assert!(matches!(top(&h), Some(PopupType::ContextMenu(_))));
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
    // With the menu bar hidden the top row reveals it instead.
    h.ctx.config.settings.interface_always_show_menu_bar = true;
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PanelTitle(ActivePanel::Right));
    h.click(x + 2, y);
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    assert!(matches!(top(&h), Some(PopupType::FoldersHistoryList(_))));
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

#[test]
fn clicking_the_command_line_focuses_it() {
    let mut h = Harness::new();
    let (x, y) = h.hit_cell(|t| *t == HitTarget::Action(crate::keybindings::Action::FocusCli));
    assert!(!h.state.cli.focused);
    h.click(x + 3, y);
    assert!(h.state.cli.focused);
}

#[test]
fn the_wheel_scrolls_the_viewer_without_a_scrollbar() {
    let mut h = Harness::new();
    let text: String = (0..200).map(|i| format!("line {i}\n")).collect();
    h.write("work/left/long.txt", text);
    h.reread();
    h.focus("long.txt").keys("@view");
    let scroll = |h: &Harness| match Some(h.state.screens.active()) {
        Some(crate::app::state::Screen::Viewer(vw)) => vw.scroll,
        _ => panic!("viewer not open"),
    };
    assert_eq!(scroll(&h), 0);
    h.wheel(10, 10, true);
    assert_eq!(scroll(&h), crate::ui::hit::WHEEL_LINES);
    h.wheel(10, 10, false);
    assert_eq!(scroll(&h), 0);
}

#[test]
fn dragging_the_border_between_the_panels_moves_the_split() {
    let mut h = panels(0);
    let (x, y) = h.hit_cell(|t| matches!(t, HitTarget::PanelSplitter { .. }));
    assert_eq!(h.ctx.config.settings.panel_split, 50);
    h.drag(
        MouseButton::Left,
        (x, y + 3),
        (x + 20, y + 3),
        KeyModifiers::NONE,
    );
    assert_eq!(h.ctx.config.settings.panel_split, 70);
    let left = h.state.panels.active;
    assert_eq!(left, ActivePanel::Left, "the drag only moved the border");
    // Never past the minimum share, and a double click evens them out.
    h.drag(
        MouseButton::Left,
        (x + 20, y + 3),
        (99, y + 3),
        KeyModifiers::NONE,
    );
    assert_eq!(h.ctx.config.settings.panel_split, 85);
    let (x, y) = h.hit_cell(|t| matches!(t, HitTarget::PanelSplitter { .. }));
    h.double_click(x, y + 3);
    assert_eq!(h.ctx.config.settings.panel_split, 50);
}

#[test]
fn split_keys_move_the_border_in_steps() {
    let mut h = Harness::new();
    h.keys("@panel_split_grow @panel_split_grow");
    assert_eq!(h.ctx.config.settings.panel_split, 60);
    h.keys("@panel_split_shrink");
    assert_eq!(h.ctx.config.settings.panel_split, 55);
    h.keys("@panel_split_reset");
    assert_eq!(h.ctx.config.settings.panel_split, 50);
}
