//! Progress and cooperative cancellation of archive work (compress,
//! extract, editing inside archives), reported straight as Transfer Engine
//! events through the job's [`JobControl`] — one progress model for every
//! long-running operation.

use crate::config::localization::t;
use crate::fs::transfer::control::{JobControl, done, failed};
use crate::fs::transfer::job::{SkippedFile, TransferResults};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// What an archive operation reports while it runs, and the results it
/// collects for the job.
pub struct ArchiveReport {
    ctl: JobControl,
    results: TransferResults,
    /// Entry count announced when the operation does not know its own.
    default_total: usize,
    announced: bool,
}

impl ArchiveReport {
    pub fn new(ctl: JobControl, default_total: usize) -> Self {
        Self {
            ctl,
            results: TransferResults::default(),
            default_total,
            announced: false,
        }
    }

    /// The first report announces the size of the job.
    fn announce(&mut self, total: usize) {
        if !self.announced {
            self.announced = true;
            let total = if total > 0 { total } else { self.default_total };
            self.ctl.scan_complete(total, 0);
        }
    }

    /// The entry `name` (number `index` of `total`; 0 when unknown) is
    /// written.
    pub fn entry(&mut self, name: &str, index: usize, total: usize) {
        self.announce(total);
        let path = PathBuf::from(name);
        self.ctl.file_started(&path, index);
        let result = done(&path, path.clone(), 0, Instant::now());
        self.ctl.file_completed(&mut self.results, result);
    }

    /// The entry `rel` was not written: something already exists there.
    pub fn skipped(&mut self, rel: &Path) {
        self.announce(0);
        let skipped = SkippedFile {
            src: rel.to_path_buf(),
            reason: t("archive_entry_skipped_exists"),
        };
        self.ctl.file_skipped(&mut self.results, skipped);
    }

    /// The operation failed on `name` (the caller returns the error).
    pub fn failed(&mut self, name: &Path, error: String) {
        self.announce(0);
        self.ctl
            .file_failed(&mut self.results, failed(name, PathBuf::new(), error));
    }

    pub fn is_cancelled(&self) -> bool {
        self.ctl.is_cancelled()
    }

    /// `Err` once the job was cancelled (cooperative cancel point).
    pub fn ensure_running(&self) -> anyhow::Result<()> {
        self.ctl.ensure_running()
    }

    /// The job's cancel flag, for callbacks that cannot borrow the report.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.ctl.is_cancelled)
    }

    pub fn into_results(self) -> TransferResults {
        self.results
    }

    /// A report that goes nowhere (tests).
    #[cfg(test)]
    pub fn detached(cancelled: bool) -> Self {
        let (event_tx, _rx) = crate::fs::transfer::events::EventSender::channel();
        let ctl = JobControl {
            job_id: uuid::Uuid::nil(),
            is_paused: Arc::default(),
            is_cancelled: Arc::new(AtomicBool::new(cancelled)),
            skip_file_flag: Arc::default(),
            event_tx,
        };
        Self::new(ctl, 0)
    }
}
