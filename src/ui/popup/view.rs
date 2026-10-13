//! Painting dialogs: the UI half of the dialog list (ARCH-02).
//!
//! Every dialog type of `app::state::popup::dialog_list!` implements
//! [`DialogView`]; [`render_dialog`] is generated from the same list, so a
//! dialog without a view does not compile.

use crate::app::context::AppContext;
use crate::app::state::popup::*;
use crate::app::state::{AppState, PopupType};
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{Frame, layout::Rect};

/// What a dialog may read while it paints.
pub struct ViewCtx<'a> {
    pub state: &'a AppState,
    pub context: &'a AppContext,
    pub theme: &'a ResolvedTheme,
    /// The whole screen (dialogs center on it).
    pub area: Rect,
    /// Left and right panel areas (for dialogs anchored to a panel).
    pub panels: (Rect, Rect),
}

impl ViewCtx<'_> {
    /// Scrollbar state, for dialogs with a scrollbar.
    pub fn scrollbar(&self) -> Option<&ScrollbarUiState> {
        Some(&self.state.scrollbar)
    }
}

/// How a dialog paints itself.
pub trait DialogView {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>);
}

impl<T: DialogView> DialogView for Box<T> {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        (**self).render(f, cx);
    }
}

/// Generates [`render_dialog`] from the dialog list.
macro_rules! define_render_dialog {
    ($($variant:ident($ty:ty),)*) => {
        /// Paints `popup`.
        pub fn render_dialog(f: &mut Frame, popup: &PopupType, cx: &ViewCtx<'_>) {
            match popup {
                $(PopupType::$variant(dialog) => DialogView::render(dialog, f, cx),)*
            }
        }
    };
}

dialog_list!(define_render_dialog);
