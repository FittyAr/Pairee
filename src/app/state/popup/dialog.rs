//! What every overlay dialog answers for itself (ARCH-02).
//!
//! [`PopupType`] is generated from one list (`dialog_list!` in `mod.rs`):
//! each variant wraps a dialog type implementing [`Dialog`], and every
//! dispatch over the open dialog (keys, outside clicks, resizing, pastes,
//! scrollbars) goes through [`PopupType::dialog`]. Painting is the UI's
//! side of the same list (`ui::popup::view::DialogView`), so adding a
//! dialog means one type, its two trait impls and one line in the list.

use super::PopupType;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crate::ui::scrollbar::ScrollTargetId;
use crossterm::event::KeyEvent;

/// What a dialog's key handler did: `Ok(Some(action))` runs `action`,
/// `Ok(None)` consumed the key, `Err(())` left it unused.
pub type KeyOutcome = Result<Option<Action>, ()>;

/// Handles a key for the dialog on top. It gets the whole state (dialogs
/// open others, change panels and close themselves) and finds its dialog
/// with [`crate::app::state::DialogStack::top_mut`].
pub type KeyHandler = fn(&mut AppState, KeyEvent, &mut AppContext) -> KeyOutcome;

/// What a scrollbar of the dialog moves.
pub enum DialogScroll<'a> {
    /// A text view: its first shown line.
    Offset(&'a mut usize),
    /// A list: its cursor and length (the view follows the cursor).
    Cursor(&'a mut usize, usize),
    /// A list whose view scrolls apart from its cursor.
    OffsetAndCursor {
        offset: &'a mut usize,
        cursor: &'a mut usize,
    },
}

/// An overlay dialog: its state plus how it reacts to input. Rendering
/// lives in the UI layer (`ui::popup::view::DialogView`).
pub trait Dialog {
    /// The function that handles keys while this dialog is on top.
    fn key_handler(&self) -> KeyHandler;

    /// A click outside closes it (as `Esc` would): menus, pickers and
    /// read-only panels. Dialogs that hold typed input or ask for a
    /// decision stay open so a stray click cannot discard them.
    fn closes_on_outside_click(&self) -> bool {
        false
    }

    /// Its content flows into whatever size it gets, so the user may
    /// resize it (by its bottom-right corner).
    fn resizable(&self) -> bool {
        false
    }

    /// The text field that currently receives typing and pastes, if any.
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        None
    }

    /// What the scrollbar `id` moves in this dialog, if it is one of its.
    fn scroll_mut(&mut self, _id: ScrollTargetId) -> Option<DialogScroll<'_>> {
        None
    }

    /// It reads every key itself (it records keys or hands them to plugin
    /// code), so the `[list]` keymap never translates them.
    fn reads_raw_keys(&self) -> bool {
        false
    }

    /// Keys it does not handle go on to the screen under it (a preview
    /// beside the panels: the panels keep their keys). Other dialogs own
    /// every key while they are open.
    fn passes_other_keys(&self) -> bool {
        false
    }
}

/// Large dialogs are boxed in [`PopupType`]; the box is the dialog.
impl<T: Dialog> Dialog for Box<T> {
    fn key_handler(&self) -> KeyHandler {
        (**self).key_handler()
    }
    fn closes_on_outside_click(&self) -> bool {
        (**self).closes_on_outside_click()
    }
    fn resizable(&self) -> bool {
        (**self).resizable()
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        (**self).focused_field_mut()
    }
    fn scroll_mut(&mut self, id: ScrollTargetId) -> Option<DialogScroll<'_>> {
        (**self).scroll_mut(id)
    }
    fn reads_raw_keys(&self) -> bool {
        (**self).reads_raw_keys()
    }
    fn passes_other_keys(&self) -> bool {
        (**self).passes_other_keys()
    }
}

/// Typed access to one dialog type inside a [`PopupType`] (generated for
/// every type of the list).
pub trait DialogKind: Dialog + Sized + Into<PopupType> {
    fn peek(popup: &PopupType) -> Option<&Self>;
    fn peek_mut(popup: &mut PopupType) -> Option<&mut Self>;
    /// The dialog, if `popup` is one.
    fn take(popup: PopupType) -> Option<Self>;
}

/// Defines [`PopupType`] from the dialog list: one variant per dialog
/// type, plus [`PopupType::dialog`] and the typed accessors.
macro_rules! define_popup_type {
    ($($variant:ident($ty:ty),)*) => {
        /// The open overlay dialog (see [`Dialog`]).
        #[derive(Debug, Clone)]
        pub enum PopupType {
            $($variant($ty),)*
        }

        impl PopupType {
            /// The dialog behind the variant: the one dispatch point.
            pub fn dialog(&self) -> &dyn Dialog {
                match self {
                    $(Self::$variant(d) => d,)*
                }
            }

            /// The dialog behind the variant, mutably.
            pub fn dialog_mut(&mut self) -> &mut dyn Dialog {
                match self {
                    $(Self::$variant(d) => d,)*
                }
            }
        }

        $(
            impl From<$ty> for PopupType {
                fn from(dialog: $ty) -> Self {
                    Self::$variant(dialog)
                }
            }

            impl DialogKind for $ty {
                fn peek(popup: &PopupType) -> Option<&Self> {
                    match popup {
                        PopupType::$variant(d) => Some(d),
                        _ => None,
                    }
                }
                fn peek_mut(popup: &mut PopupType) -> Option<&mut Self> {
                    match popup {
                        PopupType::$variant(d) => Some(d),
                        _ => None,
                    }
                }
                fn take(popup: PopupType) -> Option<Self> {
                    match popup {
                        PopupType::$variant(d) => Some(d),
                        _ => None,
                    }
                }
            }
        )*
    };
}
pub(super) use define_popup_type;
