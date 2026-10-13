//! Keys of the diff viewer shared by the Git diff and Compare files:
//! arrows / PgUp / PgDn / Home / End scroll, Tab (or `s`) switches between
//! the unified and the side-by-side layout, Esc / `q` close.

use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::popup::DiffViewer;
use crossterm::event::KeyCode;

const DIFF_SCROLL: ScrollKeys = ScrollKeys {
    vim: false,
    home_end: true,
    page: 15,
};

/// What a key did to a diff viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKey {
    Handled,
    Close,
    /// Not a viewer key.
    Other,
}

pub fn handle_key(viewer: &mut DiffViewer, code: KeyCode) -> DiffKey {
    let last = viewer.rows().saturating_sub(1);
    if scroll_key(DIFF_SCROLL, code, &mut viewer.scroll, last) {
        return DiffKey::Handled;
    }
    match code {
        KeyCode::Tab | KeyCode::Char('s' | 'S') => {
            viewer.side_by_side = !viewer.side_by_side;
            viewer.scroll = viewer.scroll.min(viewer.rows().saturating_sub(1));
            DiffKey::Handled
        }
        KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q' | 'Q') => DiffKey::Close,
        _ => DiffKey::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffDocument;

    #[test]
    fn scrolls_switches_layout_and_closes() {
        let doc = DiffDocument::from_unified("@@ -1,2 +1,2 @@\n-a\n-b\n+c\n+d\n");
        let mut viewer = DiffViewer::new(doc);
        assert_eq!(viewer.rows(), 5);
        assert_eq!(handle_key(&mut viewer, KeyCode::End), DiffKey::Handled);
        assert_eq!(viewer.scroll, 4);
        assert_eq!(handle_key(&mut viewer, KeyCode::Tab), DiffKey::Handled);
        assert!(viewer.side_by_side);
        assert_eq!(viewer.rows(), 3);
        assert_eq!(viewer.scroll, 2);
        assert_eq!(handle_key(&mut viewer, KeyCode::Char('x')), DiffKey::Other);
        assert_eq!(handle_key(&mut viewer, KeyCode::Esc), DiffKey::Close);
    }
}
