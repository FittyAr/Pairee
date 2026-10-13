//! Dialogs on top: clicks outside them, moving and resizing them, their
//! lists.

use super::top;
use crate::app::state::PopupType;
use crate::app::state::popup::CommandPalette;
use crate::app::state::popup::TransferPanel;
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;
use crossterm::event::{KeyModifiers, MouseButton};

#[test]
fn clicking_outside_a_transient_popup_closes_it() {
    let mut h = Harness::new();
    h.keys("@about");
    assert!(matches!(top(&h), Some(PopupType::About(_))));
    h.click(0, 1);
    assert!(top(&h).is_none(), "{}", h.screen_text());
}

#[test]
fn clicking_inside_a_popup_keeps_it_open() {
    let mut h = Harness::new();
    h.keys("@about");
    h.click(50, 15);
    assert!(matches!(top(&h), Some(PopupType::About(_))));
}

#[test]
fn a_dialog_with_typed_input_survives_a_stray_click() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    assert!(matches!(top(&h), Some(PopupType::MkDirPrompt(_))));
    h.click(0, 1);
    assert!(matches!(top(&h), Some(PopupType::MkDirPrompt(_))));
}

fn body(h: &mut Harness) -> ratatui::layout::Rect {
    h.render();
    h.state
        .hits
        .first(|t| (*t == HitTarget::PopupBody).then_some(()))
        .expect("dialog body")
        .0
}

#[test]
fn dialogs_move_by_their_top_border_and_recenter_on_double_click() {
    let mut h = Harness::new();
    h.keys("@about");
    let start = body(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupTitle);
    h.drag(
        MouseButton::Left,
        (x + 3, y),
        (x + 13, y + 2),
        KeyModifiers::NONE,
    );
    let moved = body(&mut h);
    assert_eq!((moved.x, moved.y), (start.x + 10, start.y + 2));
    assert!(matches!(top(&h), Some(PopupType::About(_))), "still open");
    // A click inside the moved dialog does not count as outside.
    h.click(moved.x + 2, moved.y + 3);
    assert!(top(&h).is_some());
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupTitle);
    h.double_click(x + 3, y);
    assert_eq!(body(&mut h), start);
}

#[test]
fn dialogs_stay_on_screen_and_follow_back_at_once() {
    let mut h = Harness::new();
    h.keys("@about");
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupTitle);
    h.drag(MouseButton::Left, (x + 3, y), (99, y), KeyModifiers::NONE);
    let at_edge = body(&mut h);
    assert_eq!(at_edge.right(), 100);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupTitle);
    h.drag(
        MouseButton::Left,
        (x + 3, y),
        (x + 1, y),
        KeyModifiers::NONE,
    );
    assert_eq!(body(&mut h).x, at_edge.x - 2);
}

#[test]
fn flowing_dialogs_resize_by_their_corner_and_fixed_ones_do_not() {
    let mut h = Harness::new();
    h.keys("@about");
    let start = body(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupResize);
    h.drag(
        MouseButton::Left,
        (x, y),
        (x + 6, y + 2),
        KeyModifiers::NONE,
    );
    let grown = body(&mut h);
    assert_eq!(
        (grown.width, grown.height),
        (start.width + 6, start.height + 2)
    );
    assert_eq!(
        (grown.x, grown.y),
        (start.x, start.y),
        "the top-left corner stays"
    );
    h.keys("Esc @mkdir");
    h.render();
    assert!(
        h.state
            .hits
            .first(|t| (*t == HitTarget::PopupResize).then_some(()))
            .is_none()
    );
}

fn palette_cursor(h: &Harness) -> usize {
    match top(h) {
        Some(PopupType::CommandPalette(CommandPalette { cursor_idx, .. })) => *cursor_idx,
        other => panic!("palette not open: {other:?}"),
    }
}

#[test]
fn list_rows_take_clicks_double_clicks_and_the_wheel() {
    let mut h = Harness::new();
    h.keys("@command_palette");
    let row = |steps| move |t: &HitTarget| matches!(t, HitTarget::PopupRow { steps: s, .. } if *s == steps);
    let (x, y) = h.hit_cell(row(2));
    h.click(x + 2, y);
    assert_eq!(palette_cursor(&h), 2);
    h.wheel(x + 2, y, true);
    assert_eq!(palette_cursor(&h), 5);
    h.wheel(x + 2, y, false);
    assert_eq!(palette_cursor(&h), 2);
    // Clicking inside the palette keeps it open (it records its body now).
    let (qx, qy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    h.click(qx + 4, qy);
    assert!(matches!(top(&h), Some(PopupType::CommandPalette(_))));
}

#[test]
fn double_clicking_a_list_row_runs_it() {
    let mut h = Harness::new();
    h.keys("@command_palette").text("about");
    let (x, y) = h.hit_cell(|t| matches!(t, HitTarget::PopupRow { steps: 0, .. }));
    h.double_click(x + 2, y);
    assert!(
        matches!(top(&h), Some(PopupType::About(_))),
        "{}",
        h.screen_text()
    );
}

#[test]
fn plugin_search_rows_and_pages_take_clicks() {
    let mut h = Harness::new();
    let registry: Vec<(String, String, String, String)> = (0..60)
        .map(|i| {
            (
                format!("plugin{i:02}"),
                "1.0".into(),
                "desc".into(),
                "me".into(),
            )
        })
        .collect();
    h.state.dialogs.replace(crate::app::state::PluginMenuState {
        active_tab: 1,
        all_registry: registry.clone(),
        registry,
        ..Default::default()
    });
    let cursor = |h: &Harness| match top(h) {
        Some(PopupType::PluginMenu(menu)) => menu.cursor_idx,
        other => panic!("{other:?}"),
    };
    let (x, y) = h.hit_cell(|t| matches!(t, HitTarget::PopupRow { steps: 3, .. }));
    h.click(x + 2, y);
    assert_eq!(cursor(&h), 3);
    // The page indicator turns the page.
    let (px, py) = h.locate("Pg 1/");
    h.click(px, py);
    assert!(cursor(&h) > 3, "{}", h.screen_text());
}

#[test]
fn transfer_jobs_take_clicks() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.write("work/left/b.txt", "b");
    h.reread();
    for name in ["a.txt", "b.txt"] {
        h.focus(name).keys("@copy Enter");
        let target = format!("work/right/{name}");
        h.wait_until("copy", |h| h.exists(&target));
    }
    h.keys("@toggle_transfer_panel");
    if !matches!(top(&h), Some(PopupType::TransferPanel(TransferPanel))) {
        h.keys("@toggle_transfer_panel");
    }
    assert!(
        matches!(top(&h), Some(PopupType::TransferPanel(TransferPanel))),
        "{}",
        h.screen_text()
    );
    let queue = |h: &Harness| h.state.transfer.as_ref().unwrap().queue_cursor;
    let (x, y) = h.locate("#2 ");
    h.click(x + 1, y);
    assert_eq!(queue(&h), 1);
    let (x, y) = h.locate("#1 ");
    h.click(x + 1, y + 1);
    assert_eq!(queue(&h), 0);
}
