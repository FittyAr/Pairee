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
    let last = quick_view_last_offset(state);
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
            scroll_key(QUICK_VIEW_SCROLL, key.code, &mut qv.scroll, last);
        }
        _ => return Err(()),
    }
    Ok(None)
}

/// The furthest the quick view scrolls, from what the last frame painted:
/// its scrollbar records the content length (text lines, or the rows of a
/// plugin's paragraph, list or table) and the real viewport. Nothing
/// recorded means everything fits (a scaled image, a short file, a widget
/// that does not scroll), so the view stays at the top.
fn quick_view_last_offset(state: &AppState) -> usize {
    state
        .hits
        .scrollbars()
        .into_iter()
        .find(|bar| bar.id == ScrollTargetId::QuickView)
        .map_or(0, |bar| bar.content_len.saturating_sub(bar.viewport_len))
}

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
    fn reads_raw_keys(&self) -> bool {
        true
    }
    fn passes_other_keys(&self) -> bool {
        true
    }
}
