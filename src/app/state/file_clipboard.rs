//! Pairee's own file clipboard (yank / cut, then paste into any folder),
//! as in Explorer (Ctrl+C / X / V), Vim file managers (`yy`, `p`) and yazi
//! (`y`, `x`, `p`).

use crate::fs::vfs::PanelSource;
use std::path::PathBuf;

/// What pasting does with the clipboard's items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
pub struct FileClipboard {
    pub paths: Vec<PathBuf>,
    pub mode: ClipMode,
    /// Where the items live (local disk, SFTP server, archive).
    pub source: PanelSource,
}
