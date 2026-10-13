//! Settings dialogs opened from the configuration dialog (color groups,
//! files highlighting) and the "update available" notice.

use crate::app::text_input::TextField;

/// Colors of the theme being edited.
#[derive(Debug, Clone)]
pub struct ColorGroupsDialog {
    pub cursor_idx: usize,
    /// The color being typed (Enter), if any.
    pub edit: Option<TextField>,
    pub theme: Box<crate::config::theme::Theme>,
}

/// Highlighting rules (mask and color) being edited.
#[derive(Debug, Clone)]
pub struct FilesHighlightingDialog {
    pub cursor_idx: usize,
    /// The color being typed (Enter), if any.
    pub edit: Option<TextField>,
    pub rules: Vec<crate::ui::highlight::HighlightRule>,
}

/// A newer release: its notes, Install / Ignore / Close and the download.
#[derive(Debug, Clone)]
pub struct UpdateAvailable {
    pub info: crate::update::UpdateInfo,
    pub cursor_idx: usize,
    pub install_progress: Option<f32>,
    pub error: Option<String>,
    pub scroll_y: usize,
}
