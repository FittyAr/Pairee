use crate::app::context::AppContext;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::popup::{
    CompareFoldersResult, Dialog, DialogScroll, ErrorMessage, InfoMessage, InfoPanel, KeyHandler,
    KeyOutcome, QuickViewDialog,
};
use crate::app::state::{AppState, PopupType};
use crate::ui::scrollbar::ScrollTargetId;
use crossterm::event::{KeyCode, KeyEvent};

/// Esc / Enter close a message (back to the dialog under it, if any).
pub fn handle(state: &mut AppState, key: KeyEvent, _context: &mut AppContext) -> KeyOutcome {
    if !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
        return Err(());
    }
    if state.dialogs.depth() > 1 {
        state.dialogs.pop();
    } else {
        state.dialogs.clear();
    }
    Ok(None)
}

/// Keys of the quick view: Esc closes it, PgUp / PgDn scroll it.
fn handle_quick_view(state: &mut AppState, key: KeyEvent, _context: &mut AppContext) -> KeyOutcome {
    let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top_mut() else {
        return Err(());
    };
    match key.code {
        KeyCode::Esc => {
            state.dialogs.clear();
            state.panels.quick_view_active = false;
        }
        KeyCode::PageUp | KeyCode::PageDown if qv.plugin.as_ref().is_some_and(|p| p.seekable) => {
            let page = QUICK_VIEW_SCROLL.page as i64;
            let units = if key.code == KeyCode::PageUp {
                -page
            } else {
                page
            };
            if let Some(preview) = &qv.plugin {
                crate::app::state::quick_view::plugin_preview::seek_plugin_preview(
                    &qv.path, preview, units,
                );
            }
        }
        KeyCode::PageUp | KeyCode::PageDown => {
            let last = match &qv.image_data {
                Some(img) => (img.height() as usize).div_ceil(2).saturating_sub(5),
                None => qv.content.len().saturating_sub(QUICK_VIEW_ROWS),
            };
            scroll_key(QUICK_VIEW_SCROLL, key.code, &mut qv.scroll, last);
        }
        _ => return Err(()),
    }
    Ok(None)
}

/// Visible text rows assumed when clamping the quick-view scroll.
const QUICK_VIEW_ROWS: usize = 20;

const QUICK_VIEW_SCROLL: ScrollKeys = ScrollKeys {
    vim: false,
    home_end: false,
    page: 15,
};

/// Messages close on Esc / Enter or a click outside them.
macro_rules! message_dialog {
    ($($ty:ty: resizable $resizable:literal),*) => {$(
        impl Dialog for $ty {
            fn key_handler(&self) -> KeyHandler {
                handle
            }
            fn closes_on_outside_click(&self) -> bool {
                true
            }
            fn resizable(&self) -> bool {
                $resizable
            }
        }
    )*};
}
message_dialog!(ErrorMessage: resizable false, InfoMessage: resizable false, InfoPanel: resizable true);

impl Dialog for CompareFoldersResult {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn resizable(&self) -> bool {
        true
    }
}

impl Dialog for QuickViewDialog {
    fn key_handler(&self) -> KeyHandler {
        handle_quick_view
    }
    fn scroll_mut(&mut self, id: ScrollTargetId) -> Option<DialogScroll<'_>> {
        (id == ScrollTargetId::QuickView).then_some(DialogScroll::Offset(&mut self.scroll))
    }
}
