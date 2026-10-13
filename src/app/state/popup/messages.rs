//! Read-only boxes: error and info messages, the info panel, About and the
//! Help browser.

use super::PopupType;
use std::path::PathBuf;

/// An error message (Esc / Enter dismisses it).
#[derive(Debug, Clone)]
pub struct ErrorMessage(pub String);

/// A notice (Esc / Enter dismisses it).
#[derive(Debug, Clone)]
pub struct InfoMessage(pub String);

/// Several lines of information (keymap reports, plugin notices).
#[derive(Debug, Clone)]
pub struct InfoPanel {
    pub lines: Vec<String>,
}

/// The About box.
#[derive(Debug, Clone)]
pub struct About {
    pub scroll_y: usize,
}

/// The Help browser: document list (mode 0) and document reader (mode 1).
#[derive(Debug, Clone)]
pub struct Help {
    /// 0 = list focus, 1 = reader focus.
    pub mode: usize,
    /// Core documents.
    pub docs: Vec<(String, PathBuf)>,
    /// Documents shipped by plugins.
    pub plugin_docs: Vec<(String, PathBuf)>,
    /// 0 = core help, 1 = plugins help.
    pub active_tab: usize,
    pub cursor_idx: usize,
    pub scroll_y: usize,
    pub active_content: Option<String>,
}

impl PopupType {
    /// An error message box.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(ErrorMessage(message.into()))
    }

    /// An info message box.
    pub fn info(message: impl Into<String>) -> Self {
        Self::Info(InfoMessage(message.into()))
    }
}
