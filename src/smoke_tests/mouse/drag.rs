//! Dragging files to the other panel, a folder or the command line.

use super::{panels, top};
use crate::app::state::{ActivePanel, PopupType};
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;
use crossterm::event::{KeyModifiers, MouseButton};

fn transfer_prompt(
    h: &Harness,
) -> Option<(
    crate::app::state::popup::TransferPromptOp,
    Vec<String>,
    std::path::PathBuf,
)> {
    match top(h) {
        Some(PopupType::TransferPrompt(p)) => Some((
            p.op,
            p.src_paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect(),
            p.dest_dir.clone(),
        )),
        _ => None,
    }
}

#[test]
fn dragging_a_file_to_the_other_panel_opens_the_copy_dialog() {
    use crate::app::state::popup::TransferPromptOp;
    let mut h = panels(0);
    let from = h.locate("alpha.txt");
    h.drag(MouseButton::Left, from, (80, 20), KeyModifiers::NONE);
    let (op, src, dest) = transfer_prompt(&h).expect("copy dialog");
    assert_eq!(op, TransferPromptOp::Copy);
    assert_eq!(src, ["alpha.txt"]);
    assert_eq!(dest, h.right());
    // Shift held at the drop moves instead (Shift on the press would tag a
    // range).
    h.keys("Esc");
    use crossterm::event::MouseEventKind as K;
    h.mouse(
        K::Down(MouseButton::Left),
        from.0,
        from.1,
        KeyModifiers::NONE,
    );
    h.mouse(K::Drag(MouseButton::Left), 80, 20, KeyModifiers::SHIFT);
    h.mouse(K::Up(MouseButton::Left), 80, 20, KeyModifiers::SHIFT);
    assert_eq!(
        transfer_prompt(&h).expect("move dialog").0,
        TransferPromptOp::Move
    );
}

#[test]
fn dropping_on_a_folder_targets_it_and_tagged_files_travel_together() {
    let mut h = panels(0);
    let (x, y) = h.locate("alpha.txt");
    h.right_click(x, y);
    let (bx, by) = h.locate("beta.txt");
    h.right_click(bx, by);
    let to = h.locate("sub");
    h.drag(MouseButton::Left, (x, y), to, KeyModifiers::NONE);
    let (_, mut src, dest) = transfer_prompt(&h).expect("copy dialog");
    src.sort();
    assert_eq!(src, ["alpha.txt", "beta.txt"]);
    assert_eq!(dest, h.left().join("sub"));
}

#[test]
fn drops_where_the_files_already_are_do_nothing() {
    let mut h = panels(0);
    let from = h.locate("alpha.txt");
    let to = h.locate("gamma.txt");
    h.drag(MouseButton::Left, from, to, KeyModifiers::NONE);
    assert!(top(&h).is_none());
    // Nor does a drag that Esc called off.
    use crossterm::event::MouseEventKind as K;
    h.mouse(
        K::Down(MouseButton::Left),
        from.0,
        from.1,
        KeyModifiers::NONE,
    );
    h.mouse(K::Drag(MouseButton::Left), 80, 20, KeyModifiers::NONE);
    h.keys("Esc");
    h.mouse(K::Up(MouseButton::Left), 80, 20, KeyModifiers::NONE);
    assert!(top(&h).is_none());
}

#[test]
fn dropping_on_the_command_line_inserts_the_path() {
    let mut h = panels(0);
    let from = h.locate("beta.txt");
    let (cx, cy) = h.hit_cell(|t| *t == HitTarget::Action(crate::keybindings::Action::FocusCli));
    h.drag(MouseButton::Left, from, (cx + 5, cy), KeyModifiers::NONE);
    assert!(
        h.state.cli.text().contains("beta.txt"),
        "{:?}",
        h.state.cli.text()
    );
    assert!(h.state.cli.focused);
}

#[test]
fn the_pointer_label_says_what_goes_where() {
    let mut h = panels(0);
    let from = h.locate("alpha.txt");
    use crossterm::event::MouseEventKind as K;
    h.mouse(
        K::Down(MouseButton::Left),
        from.0,
        from.1,
        KeyModifiers::NONE,
    );
    h.mouse(K::Drag(MouseButton::Left), 70, 20, KeyModifiers::NONE);
    h.mouse(K::Drag(MouseButton::Left), 71, 20, KeyModifiers::NONE);
    let copy = crate::config::localization::t("drag_copy");
    h.assert_screen(&format!("{copy} alpha.txt → right"));
}

#[test]
fn pressing_shift_alone_keeps_the_drag_going() {
    use crate::app::state::popup::TransferPromptOp;
    use crossterm::event::{KeyCode, KeyEvent, ModifierKeyCode, MouseEventKind as K};
    let mut h = panels(0);
    let from = h.locate("alpha.txt");
    h.mouse(
        K::Down(MouseButton::Left),
        from.0,
        from.1,
        KeyModifiers::NONE,
    );
    h.mouse(K::Drag(MouseButton::Left), 80, 20, KeyModifiers::NONE);
    h.send(crate::terminal::Event::Key(KeyEvent::new(
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
        KeyModifiers::SHIFT,
    )));
    h.mouse(K::Up(MouseButton::Left), 80, 20, KeyModifiers::SHIFT);
    assert_eq!(
        transfer_prompt(&h).expect("move dialog").0,
        TransferPromptOp::Move
    );
}

#[test]
fn dragging_to_a_panel_edge_scrolls_its_view_not_its_cursor() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    for i in 0..60 {
        h.write(&format!("work/right/r{i:02}.dat"), "x");
    }
    h.reread();
    let shown = |h: &mut Harness| {
        h.render();
        h.state.hits.collect(|t| match *t {
            HitTarget::PanelRow {
                side: ActivePanel::Right,
                index,
            } => Some(index),
            _ => None,
        })
    };
    let rows = shown(&mut h);
    let (first, last) = (*rows.iter().min().unwrap(), *rows.iter().max().unwrap());
    assert_eq!(first, 0);
    let (_, last_y) = h.hit_cell(|t| {
        *t == HitTarget::PanelRow {
            side: ActivePanel::Right,
            index: last,
        }
    });
    let right_cursor = h.state.panels.tabs(ActivePanel::Right).panel().cursor_index;
    let from = h.locate("a.txt");
    use crossterm::event::MouseEventKind as K;
    h.mouse(
        K::Down(MouseButton::Left),
        from.0,
        from.1,
        KeyModifiers::NONE,
    );
    h.mouse(K::Drag(MouseButton::Left), 70, last_y, KeyModifiers::NONE);
    let scrolled = shown(&mut h);
    assert_eq!(*scrolled.iter().min().unwrap(), 1, "one row further down");
    assert_eq!(
        h.state.panels.tabs(ActivePanel::Right).panel().cursor_index,
        right_cursor,
        "the cursor stays"
    );
    // Holding still there keeps scrolling with the timer.
    h.state
        .mouse
        .advance_clock(std::time::Duration::from_millis(120));
    h.send(crate::terminal::Event::Tick);
    assert_eq!(*shown(&mut h).iter().min().unwrap(), 2);
    // After the drop the view follows the cursor again.
    h.keys("Esc");
    assert_eq!(*shown(&mut h).iter().min().unwrap(), 0);
}
