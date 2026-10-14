//! The steps one file goes through in the copy phase.

use super::super::conflict_resolver::{ConflictAction, ConflictResolver};
use super::super::conflict_slot::ConflictSlot;
use super::super::control::{JobControl, failed};
use super::super::events::TransferEvent;
use super::super::job::{
    FailedFile, FileTransferResult, SkippedFile, TransferOperation, TransferResults,
};
use super::super::options::TransferOptions;
use super::super::scan::{FileMapping, guard};
use super::super::speed::SpeedReporter;
use super::retry::TransferOutcome;
use super::{Endpoints, file_copy};
use crate::config::localization::t;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

/// State of the copy phase of one job.
pub(super) struct CopyJob<'a> {
    operation: TransferOperation,
    /// The job's options, with the changes made while it runs applied
    /// before each file.
    options: TransferOptions,
    ctl: &'a JobControl,
    fs: &'a Endpoints,
    conflicts: ConflictResolver<'a>,
    results: TransferResults,
    copied: Arc<AtomicU64>,
    _speed: SpeedReporter,
}

impl<'a> CopyJob<'a> {
    pub fn new(
        operation: TransferOperation,
        options: &'a TransferOptions,
        ctl: &'a JobControl,
        (slot, fs): (&'a ConflictSlot, &'a Endpoints),
        results: TransferResults,
        total_bytes: u64,
    ) -> Self {
        let copied = Arc::new(AtomicU64::new(0));
        Self {
            operation,
            options: options.clone(),
            ctl,
            fs,
            conflicts: ConflictResolver {
                options,
                ctl,
                slot,
                src_fs: fs.src.as_ref(),
                dst_fs: fs.dst.as_ref(),
                auto: None,
            },
            results,
            _speed: SpeedReporter::spawn(ctl, Arc::clone(&copied), total_bytes),
            copied,
        }
    }

    /// Creates the destination folders and records what the scan refused
    /// (failures) or left out (skips).
    pub fn prepare(
        &mut self,
        dst_dirs: &[PathBuf],
        refused: Vec<FailedFile>,
        skipped: Vec<SkippedFile>,
    ) -> anyhow::Result<()> {
        for dir in dst_dirs {
            if self.fs.local {
                crate::fs::transfer::worker::ensure_dir(dir, &mut self.results.created_dirs);
            } else {
                self.fs.dst.mkdir_all(dir);
            }
        }
        for skip in skipped {
            self.ctl.file_skipped(&mut self.results, skip);
        }
        for failure in refused {
            self.fail(failure)?;
        }
        Ok(())
    }

    pub fn finish(self) -> TransferResults {
        self.results
    }

    /// Copies (or moves) one file.
    pub async fn file(
        &mut self,
        index: usize,
        (src, dst, size): FileMapping,
    ) -> anyhow::Result<()> {
        self.ctl.wait_if_paused().await?;
        self.ctl.live.apply_to(&mut self.options);
        if self.ctl.take_user_skip(&src, &mut self.results) {
            return Ok(());
        }
        let Some(dst) = self.destination(&src, dst).await? else {
            return Ok(());
        };
        self.ctl.file_started(&src, index);
        let start = Instant::now();
        let transfer = self.transfer(&src, &dst).await?;
        if !transfer.success {
            return self.fail(failure(&src, &dst, transfer.last_error, transfer.retries));
        }
        self.copy_metadata(&src, &dst);
        if self.operation == TransferOperation::Move
            && let Err(e) = self.fs.src.remove_file(&src)
        {
            let error = t("error_delete_source_failed")
                .replacen("{}", &src.to_string_lossy(), 1)
                .replacen("{}", &e.to_string(), 1);
            return self.fail(failure(&src, &dst, error, 0));
        }
        let done = FileTransferResult {
            size,
            src_hash: transfer.src_hash,
            dst_hash: transfer.dst_hash,
            verified: transfer.verified,
            duration: start.elapsed(),
            replaced: transfer.replaced,
            dst_stamp: self
                .fs
                .local
                .then(|| crate::fs::stamp::Stamp::of(&dst))
                .flatten(),
            src,
            dst,
        };
        self.ctl.file_completed(&mut self.results, done);
        Ok(())
    }

    async fn transfer(&self, src: &Path, dst: &Path) -> anyhow::Result<TransferOutcome> {
        file_copy::transfer_file(self.fs, src, dst, &self.options, self.ctl, &self.copied).await
    }

    fn copy_metadata(&self, src: &Path, dst: &Path) {
        file_copy::copy_metadata(self.fs, src, dst, &self.options);
    }

    /// Where `src` is written: `None` when it is not copied (the user or the
    /// conflict setting skipped it, it would replace itself, or whether
    /// something is there cannot be told). Copying a file onto itself is
    /// only possible as a renamed duplicate.
    async fn destination(
        &mut self,
        src: &Path,
        mut dst: PathBuf,
    ) -> anyhow::Result<Option<PathBuf>> {
        match self.fs.dst.stat(&dst) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Some(dst)),
            // A server that timed out (or refused the lookup) may still
            // hold a file there: never overwrite it unasked.
            Err(e) => {
                self.fail(failure(src, &dst, e.to_string(), 0))?;
                return Ok(None);
            }
            Ok(_) => {}
        }
        let action = self
            .conflicts
            .resolve(src, &mut dst, &mut self.results)
            .await?;
        if let ConflictAction::Skip = action {
            return Ok(None);
        }
        if self.fs.same_fs && guard::same_entry(self.fs.dst.as_ref(), src, &dst) {
            let error = t("error_copy_onto_itself").replacen("{}", &src.to_string_lossy(), 1);
            self.fail(failure(src, &dst, error, 0))?;
            return Ok(None);
        }
        Ok(Some(dst))
    }

    /// Records a file that could not be transferred; with "halt on error"
    /// the job fails.
    fn fail(&mut self, failed: FailedFile) -> anyhow::Result<()> {
        let error = t("transfer_halted").replacen("{}", &failed.error, 1);
        self.ctl.file_failed(&mut self.results, failed);
        if !self.options.halt_on_error {
            return Ok(());
        }
        self.ctl.emit(TransferEvent::JobFailed {
            job_id: self.ctl.job_id,
            error: error.clone(),
        });
        Err(anyhow!(error))
    }
}

fn failure(src: &Path, dst: &Path, error: String, retries: u32) -> FailedFile {
    FailedFile {
        retries,
        ..failed(src, dst.to_path_buf(), error)
    }
}
