//! What to do when a copy's destination already exists: the job's conflict
//! setting (overwrite, skip, rename, overwrite older) or, for "ask", the
//! user's answer. Shared by the local worker and by copies into archives;
//! the destination is read through the [`Vfs`] it lies on.

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

pub enum ConflictAction {
    Skip,
    Proceed,
}

/// Resolves conflicts of one job; remembers "... all" answers.
pub struct ConflictResolver<'a> {
    pub options: &'a TransferOptions,
    pub ctl: &'a JobControl,
    pub slot: &'a ConflictSlot,
    /// Filesystem of the destinations.
    pub dst_fs: &'a dyn Vfs,
    /// Answer applied to every later conflict ("Overwrite all"...).
    pub auto: Option<ConflictResolution>,
}

fn local_meta(path: &Path) -> (u64, Option<SystemTime>) {
    path.metadata()
        .map(|m| (m.len(), m.modified().ok()))
        .unwrap_or_default()
}

impl ConflictResolver<'_> {
    fn dst_meta(&self, path: &Path) -> (u64, Option<SystemTime>) {
        self.dst_fs
            .stat(path)
            .map(|e| (e.size, e.modified))
            .unwrap_or_default()
    }

    /// The setting to apply to `dst`, asking the user for "ask".
    async fn mode(&mut self, src: &Path, dst: &Path) -> anyhow::Result<String> {
        if self.options.conflict_resolution != "ask" {
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
        let (src_size, src_modified) = local_meta(src);
        let (dst_size, dst_modified) = self.dst_meta(dst);
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
            "skip" => Ok(self.skipped(src, results, "File already exists")),
            "rename" | "keep_both" => {
                *dst = resolve_filename_conflict(dst, &|p| self.dst_fs.exists(p));
                Ok(ConflictAction::Proceed)
            }
            "overwrite_older" => {
                let (_, src_time) = local_meta(src);
                let (_, dst_time) = self.dst_meta(dst);
                match (src_time, dst_time) {
                    (Some(s), Some(d)) if s <= d => {
                        Ok(self.skipped(src, results, "Destination is newer or equal"))
                    }
                    _ => Ok(ConflictAction::Proceed),
                }
            }
            _ => Ok(ConflictAction::Proceed), // Overwrite
        }
    }

    fn skipped(&self, src: &Path, results: &mut TransferResults, reason: &str) -> ConflictAction {
        results.skipped_files.push(SkippedFile {
            src: src.to_path_buf(),
            reason: format!("{reason} (skipped)"),
        });
        self.ctl.emit(TransferEvent::FileSkipped {
            job_id: self.ctl.job_id,
            file: src.to_path_buf(),
            reason: reason.to_string(),
        });
        ConflictAction::Skip
    }
}
