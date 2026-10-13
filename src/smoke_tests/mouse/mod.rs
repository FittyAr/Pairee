//! The mouse router, by area: panels, the menu bar, dialogs (and their
//! controls) and file drags.

mod dialog_controls;
mod dialogs;
mod drag;
mod menu;
mod panel;

use crate::app::state::PopupType;
use crate::test_harness::Harness;

fn top(h: &Harness) -> Option<&PopupType> {
    h.state.dialogs.top()
}

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
