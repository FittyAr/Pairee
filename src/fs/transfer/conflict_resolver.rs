//! What to do when a copy's destination already exists: the job's conflict
//! setting (overwrite, skip, rename, overwrite older) or, for "ask", the
//! user's answer. Shared by local copies, SFTP copies and copies into
//! archives; both sides are read through the [`Vfs`] they lie on.

use super::conflict::{ConflictInfo, ConflictResolution, resolve_filename_conflict};
use super::conflict_slot::ConflictSlot;
use super::control::JobControl;
use super::events::TransferEvent;
use super::job::{SkippedFile, TransferResults};
use super::options::TransferOptions;
use crate::fs::vfs::Vfs;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::SystemTime;

/// Conflict settings applied without asking.
const KNOWN_MODES: &[&str] = &[
    "overwrite",
    "overwrite_older",
    "skip",
    "rename",
    "keep_both",
];

pub enum ConflictAction {
    Skip,
    Proceed,
}

/// Resolves conflicts of one job; remembers "... all" answers.
pub struct ConflictResolver<'a> {
    pub options: &'a TransferOptions,
    pub ctl: &'a JobControl,
    pub slot: &'a ConflictSlot,
    /// Filesystem of the sources.
    pub src_fs: &'a dyn Vfs,
    /// Filesystem of the destinations.
    pub dst_fs: &'a dyn Vfs,
    /// Answer applied to every later conflict ("Overwrite all"...).
    pub auto: Option<ConflictResolution>,
}

/// Size and modification time of `path` on `fs` (links followed).
fn meta_on(fs: &dyn Vfs, path: &Path) -> (u64, Option<SystemTime>) {
    fs.metadata(path)
        .map(|e| (e.size, e.modified))
        .unwrap_or_default()
}

impl ConflictResolver<'_> {
    /// The setting to apply to `dst`, asking the user for "ask".
    async fn mode(&mut self, src: &Path, dst: &Path) -> anyhow::Result<String> {
        // A setting this code does not know (a hand-edited config) asks
        // instead of falling through to an overwrite.
        if KNOWN_MODES.contains(&self.options.conflict_resolution.as_str()) {
            return Ok(self.options.conflict_resolution.clone());
        }
        let chosen = match self.auto {
            Some(answer) => answer,
            None => self.ask(src, dst).await?,
        };
        Ok(match chosen {
            ConflictResolution::Overwrite | ConflictResolution::OverwriteAll => "overwrite",
            ConflictResolution::OverwriteOlder | ConflictResolution::OverwriteOlderAll => {
                "overwrite_older"
            }
            ConflictResolution::Rename
            | ConflictResolution::RenameAll
            | ConflictResolution::KeepBoth => "rename",
            ConflictResolution::Cancel => {
                self.ctl.is_cancelled.store(true, Ordering::SeqCst);
                return Err(anyhow!("Job cancelled"));
            }
            _ => "skip",
        }
        .to_string())
    }

    /// Shows the conflict dialog and waits (without polling) for the answer.
    async fn ask(&mut self, src: &Path, dst: &Path) -> anyhow::Result<ConflictResolution> {
        self.slot.reset();
        let (src_size, src_modified) = meta_on(self.src_fs, src);
        let (dst_size, dst_modified) = meta_on(self.dst_fs, dst);
        self.ctl.emit(TransferEvent::ConflictDetected {
            job_id: self.ctl.job_id,
            file: dst.to_path_buf(),
            conflict: ConflictInfo {
                src_path: src.to_path_buf(),
                dst_path: dst.to_path_buf(),
                src_size,
                dst_size,
                src_modified,
                dst_modified,
            },
        });
        let Some(answer) = self.slot.wait(self.ctl.is_cancelled.as_ref()).await else {
            return Err(anyhow!("Job cancelled"));
        };
        self.slot.reset();
        if matches!(
            answer,
            ConflictResolution::OverwriteAll
                | ConflictResolution::OverwriteOlderAll
                | ConflictResolution::SkipAll
                | ConflictResolution::RenameAll
        ) {
            self.auto = Some(answer);
        }
        Ok(answer)
    }

    /// Decides for the existing destination `dst` of `src`; a rename
    /// updates `dst` to a free name.
    pub async fn resolve(
        &mut self,
        src: &Path,
        dst: &mut PathBuf,
        results: &mut TransferResults,
    ) -> anyhow::Result<ConflictAction> {
        match self.mode(src, dst).await?.as_str() {
            "skip" => Ok(self.skipped(src, results, "transfer_skip_exists")),
            "rename" | "keep_both" => {
                *dst = resolve_filename_conflict(dst, &|p| self.dst_fs.exists(p));
                Ok(ConflictAction::Proceed)
            }
            "overwrite_older" => {
                let (_, src_time) = meta_on(self.src_fs, src);
                let (_, dst_time) = meta_on(self.dst_fs, dst);
                match (src_time, dst_time) {
                    (Some(s), Some(d)) if s <= d => {
                        Ok(self.skipped(src, results, "transfer_skip_newer"))
                    }
                    _ => Ok(ConflictAction::Proceed),
                }
            }
            "overwrite" => Ok(ConflictAction::Proceed),
            _ => Ok(self.skipped(src, results, "transfer_skip_exists")),
        }
    }

    /// Records `src` as skipped for the reason with translation `key`.
    fn skipped(&self, src: &Path, results: &mut TransferResults, key: &str) -> ConflictAction {
        let skipped = SkippedFile {
            src: src.to_path_buf(),
            reason: crate::config::localization::t(key),
        };
        self.ctl.file_skipped(results, skipped);
        ConflictAction::Skip
    }
}
