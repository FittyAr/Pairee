//! Runs a rename plan through a [`RenameBackend`] and undoes it on failure.

use super::names::TargetFs;
use super::plan::Step;
use crate::fs::ssh::SharedSshClient;
use std::path::{Path, PathBuf};

/// Where the renames happen (Strategy): the local filesystem or an SFTP
/// server. Both use the primitives the rest of Pairee already renames with.
pub trait RenameBackend {
    fn rename(&self, from: &Path, to: &Path) -> Result<(), String>;
    /// `true` when an entry (of any kind) exists at `path`.
    fn exists(&self, path: &Path) -> bool;
}

/// The local filesystem (`std::fs::rename`, as the single-file rename).
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

impl RenameBackend for LocalFs {
    fn rename(&self, from: &Path, to: &Path) -> Result<(), String> {
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }

    fn exists(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path).is_ok()
    }
}

/// A remote panel (SFTP rename, as the same-server fast move).
impl RenameBackend for SharedSshClient {
    fn rename(&self, from: &Path, to: &Path) -> Result<(), String> {
        self.rename_move(from, to).map_err(|e| e.to_string())
    }

    fn exists(&self, path: &Path) -> bool {
        SharedSshClient::exists(self, path)
    }
}

/// The step that failed and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameFailure {
    pub path: PathBuf,
    pub error: String,
}

/// Outcome of [`execute`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenameReport {
    /// Steps that ran and were kept.
    pub completed: usize,
    pub failure: Option<RenameFailure>,
    /// Steps that could not be undone after the failure (`(from, to)` as
    /// they were applied); empty when the rollback was complete.
    pub not_rolled_back: Vec<Step>,
}

impl RenameReport {
    pub fn is_success(&self) -> bool {
        self.failure.is_none()
    }
}

/// Applies `steps` in order. A step never overwrites: when its target
/// exists (and is not the source itself, as in a case-only rename) it fails.
/// On the first failure every applied step is undone in reverse order.
pub fn execute(steps: &[Step], backend: &dyn RenameBackend, fs: TargetFs) -> RenameReport {
    let mut applied: Vec<&Step> = Vec::with_capacity(steps.len());
    for step in steps {
        let same_entry = fs.path_key(&step.from) == fs.path_key(&step.to);
        // The target may have appeared since the preview was computed.
        let result = if !same_entry && backend.exists(&step.to) {
            Err(crate::config::localization::t("multi_rename_target_exists"))
        } else {
            backend.rename(&step.from, &step.to)
        };
        if let Err(error) = result {
            let not_rolled_back = rollback(&applied, backend);
            return RenameReport {
                completed: 0,
                failure: Some(RenameFailure {
                    path: step.from.clone(),
                    error,
                }),
                not_rolled_back,
            };
        }
        applied.push(step);
    }
    RenameReport {
        completed: applied.len(),
        failure: None,
        not_rolled_back: Vec::new(),
    }
}

/// Undoes `applied` (newest first); returns the steps that stayed applied.
fn rollback(applied: &[&Step], backend: &dyn RenameBackend) -> Vec<Step> {
    applied
        .iter()
        .rev()
        .filter(|step| backend.rename(&step.to, &step.from).is_err())
        .map(|step| (*step).clone())
        .collect()
}
