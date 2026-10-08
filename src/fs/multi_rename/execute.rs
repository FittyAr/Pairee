//! Runs a rename plan through the panel's [`Vfs`] (local or SFTP) and
//! undoes it on failure.

use super::names::TargetFs;
use super::plan::Step;
use crate::fs::vfs::Vfs;
use std::path::PathBuf;

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
pub fn execute(steps: &[Step], backend: &dyn Vfs, fs: TargetFs) -> RenameReport {
    let mut applied: Vec<&Step> = Vec::with_capacity(steps.len());
    for step in steps {
        let same_entry = fs.path_key(&step.from) == fs.path_key(&step.to);
        // The target may have appeared since the preview was computed.
        let result = if !same_entry && backend.exists(&step.to) {
            Err(crate::config::localization::t("multi_rename_target_exists"))
        } else {
            backend
                .rename(&step.from, &step.to)
                .map_err(|e| e.to_string())
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
fn rollback(applied: &[&Step], backend: &dyn Vfs) -> Vec<Step> {
    applied
        .iter()
        .rev()
        .filter(|step| backend.rename(&step.to, &step.from).is_err())
        .map(|step| (*step).clone())
        .collect()
}
