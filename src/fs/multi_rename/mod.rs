//! Multi-rename tool core (no UI): masks with placeholders, counter, search
//! & replace, case transform, live preview with conflict detection, safe
//! ordering (swaps through temporary names) and execution with rollback.
//!
//! Flow: [`RenameRules::compile`] → [`Preview::build`] → [`Preview::moves`]
//! → [`plan`] → [`execute`] on a [`RenameBackend`] (local or SFTP).

mod execute;
mod mask;
mod names;
mod plan;
mod preview;
mod rules;

#[cfg(test)]
mod plan_tests;
#[cfg(test)]
mod tests;

pub use execute::{LocalFs, RenameBackend, RenameFailure, RenameReport, execute};
pub use names::TargetFs;
pub use plan::{Step, plan};
pub use preview::{Preview, PreviewRow};
pub use rules::{CaseMode, Counter, RenameRules};

use std::path::PathBuf;
use std::time::SystemTime;

/// A file or folder to rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSource {
    pub path: PathBuf,
    pub is_dir: bool,
    pub modified: Option<SystemTime>,
}

impl RenameSource {
    /// Current file name.
    pub fn name(&self) -> String {
        crate::fs::file_name_lossy(&self.path)
    }

    /// `(name, extension)`. Folders and dot-files without another dot have
    /// no extension.
    pub fn split_name(&self) -> (String, String) {
        let name = self.name();
        if self.is_dir {
            return (name, String::new());
        }
        match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), ext.to_string()),
            _ => (name, String::new()),
        }
    }

    /// Name of the folder that contains the source.
    pub fn parent_name(&self) -> String {
        self.path
            .parent()
            .map(crate::fs::file_name_lossy)
            .unwrap_or_default()
    }
}
