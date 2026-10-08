//! Editor behaviour derived from the user's settings.

use crate::config::settings::{Settings, TabExpansion};

/// The subset of [`Settings`] that changes how the editor edits text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorOptions {
    pub tab_size: usize,
    pub tab_expansion: TabExpansion,
    pub auto_indent: bool,
    /// Read-only files open locked (no edits allowed).
    pub lock_read_only: bool,
}

impl From<&Settings> for EditorOptions {
    fn from(s: &Settings) -> Self {
        Self {
            tab_size: (s.editor_tab_size as usize).max(1),
            tab_expansion: s.editor_expand_tabs,
            auto_indent: s.editor_auto_indent,
            lock_read_only: s.editor_lock_editing_readonly,
        }
    }
}
