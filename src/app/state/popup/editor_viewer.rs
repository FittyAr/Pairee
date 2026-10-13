//! Dialogs of the editor and viewer screens: find, "save as", overwrite
//! confirmation and the encoding selector.

use super::TextSearchState;
use crate::app::text_input::TextField;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

/// Find dialog of the editor (Ctrl+F / F7).
#[derive(Debug, Clone, Default)]
pub struct EditorSearchPrompt(pub TextSearchState);

/// Find dialog of the viewer (F7).
#[derive(Debug, Clone, Default)]
pub struct ViewerSearchPrompt(pub TextSearchState);

/// Both find dialogs are the shared [`TextSearchState`].
macro_rules! search_state {
    ($($ty:ty),*) => {$(
        impl Deref for $ty {
            type Target = TextSearchState;
            fn deref(&self) -> &TextSearchState {
                &self.0
            }
        }

        impl DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut TextSearchState {
                &mut self.0
            }
        }
    )*};
}
search_state!(EditorSearchPrompt, ViewerSearchPrompt);

/// "Save as" path entry for the built-in editor (Shift+F2).
#[derive(Debug, Clone)]
pub struct EditorSaveAsPrompt {
    pub input: TextField,
}

/// Saving would overwrite a file changed on disk or an existing target.
#[derive(Debug, Clone)]
pub struct EditorConfirmOverwrite {
    pub target: PathBuf,
    pub reason: crate::app::editor::open::OverwriteReason,
}

/// Encoding selector of the viewer (F8); the cursor indexes `fs::text::ENCODINGS`.
#[derive(Debug, Clone)]
pub struct ViewerEncoding {
    pub cursor_idx: usize,
}
