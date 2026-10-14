//! Quick view beside the panels: the panels keep their keys, and dialogs
//! opened while it shows are not closed by it.

use crate::app::state::PopupType;
use crate::app::state::popup::MkDirPrompt;
use crate::test_harness::Harness;

fn quick_view_path(h: &Harness) -> Option<std::path::PathBuf> {
    match h.state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => Some(qv.path.clone()),
        _ => None,
    }
}

#[test]
fn panel_keys_and_dialogs_work_while_the_quick_view_shows() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.write("work/left/b.txt", "b");
    h.reread().focus("a.txt").keys("@quick_view");
    assert_eq!(quick_view_path(&h), Some(h.left().join("a.txt")));

    h.keys("Down");
    assert_eq!(h.cursor_name(), "b.txt", "the panel moved");
    assert_eq!(quick_view_path(&h), Some(h.left().join("b.txt")));

    h.keys("@mkdir");
    assert!(
        matches!(
            h.state.dialogs.top(),
            Some(PopupType::MkDirPrompt(MkDirPrompt { .. }))
        ),
        "{:?}",
        h.state.dialogs.top()
    );
    h.keys("Esc");
    assert_eq!(quick_view_path(&h), Some(h.left().join("b.txt")), "back");
    h.keys("Esc");
    assert!(h.state.dialogs.top().is_none());
    assert!(!h.state.panels.quick_view_active);
}
