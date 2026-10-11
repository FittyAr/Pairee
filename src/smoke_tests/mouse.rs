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
    // With the menu bar hidden the top row reveals it instead.
    h.ctx.config.settings.interface_always_show_menu_bar = true;
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

/// Row of the first item of menu `menu` that `pick` accepts.
fn menu_row(
    h: &Harness,
    menu: usize,
    pick: impl Fn(&crate::ui::menu::MenuItemData) -> bool,
) -> usize {
    crate::ui::menu::get_menu_items(menu, &h.state, &h.ctx.resolver, &h.ctx.config.settings)
        .iter()
        .position(pick)
        .expect("menu row")
}

/// Points at the hidden menu bar so its titles show.
fn reveal_menu(h: &mut Harness) {
    h.hover(1, 0);
}

fn open_menu(h: &Harness) -> Option<(usize, Option<usize>, Option<usize>)> {
    match top(h) {
        Some(&PopupType::Menu {
            active_menu_idx,
            active_item_idx,
            active_submenu_idx,
            ..
        }) => Some((active_menu_idx, active_item_idx, active_submenu_idx)),
        _ => None,
    }
}

#[test]
fn menu_titles_open_and_close_their_drop_downs() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(1));
    h.click(x + 1, y);
    assert_eq!(open_menu(&h), Some((1, Some(0), None)));
    // The drop-down opens under its own title.
    let (item_x, _) = h.hit_cell(|t| matches!(t, HitTarget::MenuItem { menu: 1, .. }));
    assert!(item_x > x, "drop-down at {item_x}, title at {x}");
    // Another title switches menus; the open one closes it.
    let (x2, _) = h.hit_cell(|t| *t == HitTarget::MenuTitle(3));
    h.click(x2 + 1, y);
    assert_eq!(open_menu(&h).map(|m| m.0), Some(3));
    h.click(x2 + 1, y);
    assert!(top(&h).is_none());
}

#[test]
fn clicking_a_menu_item_runs_it() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(1));
    h.click(x + 1, y);
    let row = menu_row(&h, 1, |item| {
        item.action == Some(crate::keybindings::Action::MkDir)
    });
    let (ix, iy) = h.hit_cell(|t| *t == HitTarget::MenuItem { menu: 1, row });
    h.click(ix + 2, iy);
    assert!(
        matches!(top(&h), Some(PopupType::MkDirPrompt { .. })),
        "{}",
        h.screen_text()
    );
}

#[test]
fn hovering_moves_the_menu_highlight_and_opens_submenus() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(0));
    h.click(x + 1, y);
    let row = menu_row(&h, 0, |item| item.submenu_idx.is_some());
    let (ix, iy) = h.hit_cell(|t| *t == HitTarget::MenuItem { menu: 0, row });
    h.hover(ix + 2, iy);
    let (menu, item, submenu) = open_menu(&h).expect("menu open");
    assert_eq!((menu, item), (0, Some(row)));
    let sub = submenu.expect("submenu opened on hover");
    // Hovering another title switches the drop-down.
    let (x2, _) = h.hit_cell(|t| *t == HitTarget::MenuTitle(4));
    h.hover(x2 + 1, y);
    assert_eq!(open_menu(&h), Some((4, Some(0), None)));
    let _ = sub;
}

#[test]
fn clicking_outside_the_menu_closes_it() {
    let mut h = panels(0);
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(2));
    h.click(x + 1, y);
    assert!(open_menu(&h).is_some());
    // A blank row of the right panel, clear of the drop-down.
    h.click(90, 20);
    assert!(top(&h).is_none());
    // The click only closed the menu.
    assert_eq!(h.state.panels.active, ActivePanel::Left);
}

#[test]
fn function_keys_on_the_bar_press_their_key() {
    let mut h = Harness::new();
    let (x, y) = h.hit_cell(|t| {
        matches!(
            t,
            HitTarget::FKey {
                number: 7,
                modifiers
            } if modifiers.is_empty()
        )
    });
    h.click(x + 1, y);
    assert!(
        matches!(top(&h), Some(PopupType::MkDirPrompt { .. })),
        "{}",
        h.screen_text()
    );
}

#[test]
fn clicking_the_command_line_focuses_it() {
    let mut h = Harness::new();
    let (x, y) = h.hit_cell(|t| *t == HitTarget::Action(crate::keybindings::Action::FocusCli));
    assert!(!h.state.cli_focused);
    h.click(x + 3, y);
    assert!(h.state.cli_focused);
}

#[test]
fn the_hidden_menu_bar_shows_while_pointed_at() {
    let mut h = panels(0);
    h.hover(3, 0);
    assert_eq!(open_menu(&h), Some((0, None, None)));
    h.hover(3, 6);
    assert!(top(&h).is_none(), "leaving the bar hides it again");
    // A click on the top row opens the menu under the pointer.
    let (x, _) = {
        h.hover(3, 0);
        h.hit_cell(|t| *t == HitTarget::MenuTitle(1))
    };
    h.hover(3, 6);
    h.click(x + 1, 0);
    assert_eq!(open_menu(&h), Some((1, Some(0), None)));
}

#[test]
fn the_wheel_scrolls_the_viewer_without_a_scrollbar() {
    let mut h = Harness::new();
    let text: String = (0..200).map(|i| format!("line {i}\n")).collect();
    h.write("work/left/long.txt", text);
    h.reread();
    h.focus("long.txt").keys("@view");
    let scroll = |h: &Harness| match h.state.screens.get(h.state.active_screen_idx) {
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
    assert!(
        matches!(top(&h), Some(PopupType::About { .. })),
        "still open"
    );
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
        Some(PopupType::CommandPalette { cursor_idx, .. }) => *cursor_idx,
        other => panic!("palette not open: {other:?}"),
    }
}

#[test]
fn list_rows_take_clicks_double_clicks_and_the_wheel() {
    let mut h = Harness::new();
    h.keys("@command_palette");
    let row = |steps| move |t: &HitTarget| *t == HitTarget::PopupRow { steps };
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
    assert!(matches!(top(&h), Some(PopupType::CommandPalette { .. })));
}

#[test]
fn double_clicking_a_list_row_runs_it() {
    let mut h = Harness::new();
    h.keys("@command_palette").text("about");
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupRow { steps: 0 });
    h.double_click(x + 2, y);
    assert!(
        matches!(top(&h), Some(PopupType::About { .. })),
        "{}",
        h.screen_text()
    );
}

/// Cell of the first dialog control showing `label` on screen.
fn control(h: &mut Harness, label: &str) -> (u16, u16) {
    h.render();
    let (x, y) = h.locate(label);
    (x + 1, y)
}

#[test]
fn form_controls_toggle_type_and_press_buttons() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    let multiple = |h: &Harness| match top(h) {
        Some(PopupType::MkDirPrompt {
            process_multiple, ..
        }) => *process_multiple,
        other => panic!("mkdir closed: {other:?}"),
    };
    let before = multiple(&h);
    let (x, y) = control(&mut h, "[ ]");
    h.click(x, y);
    assert_ne!(multiple(&h), before, "the checkbox row toggles");
    // Back in the field, typing goes to it; then OK creates the folder.
    let (fx, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    h.click(fx + 10, fy);
    h.text("made_by_mouse");
    let ok = crate::config::localization::t("btn_ok_bracket");
    let (x, y) = control(&mut h, &ok);
    h.click(x, y);
    assert!(top(&h).is_none(), "{}", h.screen_text());
    assert!(h.exists("work/left/made_by_mouse"));
}

#[test]
fn a_cancel_button_closes_its_form() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    let cancel = crate::config::localization::t("btn_cancel_bracket");
    let (x, y) = control(&mut h, &cancel);
    h.click(x, y);
    assert!(top(&h).is_none());
}

#[test]
fn clicking_in_a_field_moves_its_cursor() {
    let mut h = Harness::new();
    h.write("work/left/report.txt", "r");
    h.reread();
    h.focus("report.txt").keys("@rename");
    let (fx, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    let text_x = match h.state.hits.at(fx, fy).map(|hit| hit.target) {
        Some(HitTarget::PopupField { text_x, .. }) => text_x,
        other => panic!("{other:?}"),
    };
    h.click(text_x + 3, fy);
    let cursor = match top(&h) {
        Some(PopupType::RenamePrompt { input, .. }) => input.cursor(),
        other => panic!("{other:?}"),
    };
    assert_eq!(cursor, 3);
}

#[test]
fn copy_dialog_option_rows_toggle_on_click() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread();
    h.focus("a.txt").keys("@copy");
    let flags = |h: &Harness| match top(h) {
        Some(PopupType::TransferPrompt(p)) => {
            p.flags().into_iter().map(|(on, _)| on).collect::<Vec<_>>()
        }
        other => panic!("{other:?}"),
    };
    let before = flags(&h);
    let (x, y) = control(&mut h, "[");
    h.click(x, y);
    assert_ne!(flags(&h), before, "{}", h.screen_text());
}
