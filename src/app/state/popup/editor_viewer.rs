//! Dialogs of the editor and viewer screens: the viewer's find dialog,
//! "save as", overwrite confirmation and the encoding selector (the
//! editor's find / replace dialog is in `editor_find.rs`).

use super::TextSearchState;
use crate::app::text_input::TextField;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

/// Find dialog of the viewer (F7).
#[derive(Debug, Clone, Default)]
pub struct ViewerSearchPrompt(pub TextSearchState);

impl Deref for ViewerSearchPrompt {
    type Target = TextSearchState;
    fn deref(&self) -> &TextSearchState {
        &self.0
    }
}

impl DerefMut for ViewerSearchPrompt {
    fn deref_mut(&mut self) -> &mut TextSearchState {
        &mut self.0
    }
}

/// "Save as" path entry for the built-in editor (Shift+F2).
#[derive(Debug, Clone)]
pub struct EditorSaveAsPrompt {
    pub input: TextField,
}

/// Saving would overwrite a file changed on disk or an existing target.
#[derive(Debug, Clone)]
pub struct EditorConfirmOverwrite {
    /// The local file saved (a remote copy's temporary file).
    pub target: PathBuf,
    /// The file named to the user (a remote copy's original).
    pub shown: PathBuf,
    pub reason: crate::app::editor::open::OverwriteReason,
}

/// Encoding selector of the viewer (F8); the cursor indexes `fs::text::ENCODINGS`.
#[derive(Debug, Clone)]
pub struct ViewerEncoding {
    pub cursor_idx: usize,
}
