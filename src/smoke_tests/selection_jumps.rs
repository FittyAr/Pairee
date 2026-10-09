//! Keymap actions added for the Vim / yazi / standard presets: delete
//! modes, select all, visual selection, view and sort cycling, focus.

use crate::app::state::{ActivePanel, PanelViewMode, PopupType, SortField};
use crate::keybindings::Action;
use crate::test_harness::Harness;

fn with_files() -> Harness {
    let mut h = Harness::new();
    for name in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        h.write(&format!("work/left/{name}"), name);
    }
    h.reread();
    h
}

fn selected(h: &Harness) -> usize {
    h.state.get_active_panel().selected_paths.len()
}

#[test]
fn trash_and_permanent_delete_say_where_items_go() {
    let mut h = with_files();
    h.focus("a.txt");
    for (action, to_trash_expected) in [(Action::Trash, true), (Action::DeletePermanent, false)] {
        h.dispatch(action);
        match h.state.dialogs.top() {
            Some(PopupType::ConfirmDelete { to_trash, .. }) => {
                assert_eq!(*to_trash, to_trash_expected, "{action:?}")
            }
            other => panic!("{action:?}: expected ConfirmDelete, got {other:?}"),
        }
        h.keys("Esc");
    }
}

#[test]
fn select_all_unselect_all_and_restore() {
    let mut h = with_files();
    h.dispatch(Action::SelectAll);
    assert_eq!(selected(&h), 4, "`..` is not selected");
    h.dispatch(Action::UnselectAll);
    assert_eq!(selected(&h), 0);
    h.dispatch(Action::RestoreSelection);
    assert_eq!(selected(&h), 4);
}

#[test]
fn visual_mode_selects_the_range_the_cursor_covers() {
    let mut h = with_files();
    h.focus("a.txt");
    h.dispatch(Action::VisualMode);
    h.dispatch(Action::MoveDown);
    h.dispatch(Action::MoveDown);
    assert_eq!(selected(&h), 3);
    h.dispatch(Action::MoveUp);
    assert_eq!(selected(&h), 2);
    h.dispatch(Action::Unfocus);
    h.dispatch(Action::MoveDown);
    assert_eq!(selected(&h), 2, "leaving visual mode keeps the selection");
}

#[test]
fn view_and_sort_cycle_and_wrap() {
    let mut h = with_files();
    let panel = |h: &Harness| {
        (
            h.state.get_active_panel().view_mode,
            h.state.get_active_panel().sort_field,
        )
    };
    assert_eq!(panel(&h), (PanelViewMode::Full, SortField::Name));
    h.dispatch(Action::CyclePanelView);
    h.dispatch(Action::CycleSort);
    assert_eq!(panel(&h), (PanelViewMode::Wide, SortField::Extension));
    for _ in 0..8 {
        h.dispatch(Action::CyclePanelView);
    }
    assert_eq!(panel(&h).0, PanelViewMode::Full);
}

#[test]
fn focus_actions_pick_a_side() {
    let mut h = with_files();
    h.dispatch(Action::FocusRightPanel);
    assert_eq!(h.state.panels.active, ActivePanel::Right);
    h.dispatch(Action::FocusLeftPanel);
    assert_eq!(h.state.panels.active, ActivePanel::Left);
}

#[test]
fn rename_basename_puts_the_cursor_before_the_extension() {
    let mut h = with_files();
    h.write("work/left/report.tar.gz", "r");
    h.write("work/left/Makefile", "b");
    h.reread();
    for (name, cursor) in [
        ("report.tar.gz", "report.tar".len()),
        ("Makefile", "Makefile".len()),
    ] {
        h.focus(name);
        h.dispatch(Action::RenameBasename);
        match h.state.dialogs.top() {
            Some(PopupType::RenamePrompt { input, .. }) => {
                assert_eq!(input.cursor(), cursor, "{name}")
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
        h.keys("Esc");
    }
}

#[test]
fn create_makes_a_file_or_a_folder() {
    let mut h = with_files();
    h.dispatch(Action::Create);
    h.text("notes.md").keys("Enter");
    h.dispatch(Action::Create);
    h.text("docs/").keys("Enter");
    h.dispatch(Action::NewFile);
    h.text("empty").keys("Enter");
    assert!(h.root().join("work/left/notes.md").is_file());
    assert!(h.root().join("work/left/docs").is_dir());
    assert!(h.root().join("work/left/empty").is_file());
}
