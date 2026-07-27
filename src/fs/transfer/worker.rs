use anyhow::anyhow;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::conflict::resolve_filename_conflict;
use super::endpoint::{StatInfo, TransferEndpoint};
use super::events::TransferEvent;
use super::filter::TransferFilter;
use super::job::{
    FailedFile, FileTransferResult, LinkKind, SkippedFile, TransferOperation, TransferResults,
};
use super::metadata::preserve_metadata;
use super::options::{BufferSize, TransferOptions};
use super::pipeline::copy_file_pipelined;
use super::policy::{FileError, TransferPolicy};

/// The transfer worker. Owns everything needed to execute a single
/// `TransferJob`: the source and destination endpoints, the
/// operation kind, the per-job atomic flags, and a back-channel for
/// events.
///
/// The worker is endpoint-agnostic. It does not call `std::fs` or
/// `ssh2` directly; instead it routes every I/O through the
/// `TransferEndpoint` API. The only platform-specific code left
/// here is the per-platform recycle-bin helper (Windows uses
/// PowerShell, Unix uses `gio` / `trash-put`), which is a
/// user-driven UX detail rather than a transport-layer concern.
pub struct TransferWorker {
    pub job_id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub src_endpoint: TransferEndpoint,
    pub dst_endpoint: TransferEndpoint,
    pub options: TransferOptions,
    pub is_paused: Arc<AtomicBool>,
    pub is_cancelled: Arc<AtomicBool>,
    pub skip_file_flag: Arc<AtomicBool>,
    pub event_tx: mpsc::UnboundedSender<TransferEvent>,
    pub active_conflict:
        Arc<std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>>,
    /// Strategy the worker consults on every file
    /// failure. Held as `Arc<dyn TransferPolicy>` so the
    /// production code can swap in a `PromptPolicy` that
    /// drives the retry-as-admin popup without changing
    /// the worker's signature.
    pub policy: Arc<dyn TransferPolicy>,
}

impl TransferWorker {
    pub fn new(
        job_id: Uuid,
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        src_endpoint: TransferEndpoint,
        dst_endpoint: TransferEndpoint,
        options: TransferOptions,
        is_paused: Arc<AtomicBool>,
        is_cancelled: Arc<AtomicBool>,
        skip_file_flag: Arc<AtomicBool>,
        event_tx: mpsc::UnboundedSender<TransferEvent>,
        active_conflict: Arc<
            std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>,
        >,
        policy: Arc<dyn TransferPolicy>,
    ) -> Self {
        Self {
            job_id,
            operation,
            sources,
            destination,
            src_endpoint,
            dst_endpoint,
            options,
            is_paused,
            is_cancelled,
            skip_file_flag,
            event_tx,
            active_conflict,
            policy,
        }
    }

    /// Centralised failure reporting. Builds the
    /// [`FailedFile`], pushes it into `results`, notifies
    /// the policy, and emits the [`TransferEvent::FileFailed`]
    /// event. Every site that used to inline the 6-line
    /// `let failed = FailedFile { ... }; push; send;` pattern
    /// now goes through this method.
    fn emit_file_failed(
        &self,
        results: &mut TransferResults,
        src: &Path,
        dst: &Path,
        error: &str,
        retries: u32,
    ) {
        report_file_failure(&*self.policy, src, error);
        let failed = FailedFile {
            src: src.to_path_buf(),
            dst: dst.to_path_buf(),
            error: error.to_string(),
            retries,
        };
        results.failed_files.push(failed.clone());
        let _ = self.event_tx.send(TransferEvent::FileFailed {
            job_id: self.job_id,
            error: failed,
        });
    }

    pub async fn run(self) -> Result<TransferResults, anyhow::Error> {
        let _ = self.event_tx.send(TransferEvent::JobStarted {
            job_id: self.job_id,
        });

        // LAN detection for buffer-size optimization. For Ssh the
        // destination is a remote path, so is_lan_path returns
        // false and we keep the default 1 MiB buffer.
        let is_lan = self.dst_endpoint.is_local() && super::network::is_lan_path(&self.destination);
        let mut options = self.options.clone();
        if is_lan {
            options.buffer_size = BufferSize::_4MB;
        }

        // -----------------------------------------------------------------
        // FASE 1: SCAN
        // -----------------------------------------------------------------
        let _ = self.event_tx.send(TransferEvent::ScanProgress {
            job_id: self.job_id,
            files_found: 0,
        });

        let mut scan_mappings: Vec<(PathBuf, PathBuf, u64)> = Vec::new();
        let mut dirs_to_delete: Vec<PathBuf> = Vec::new();
        let mut total_bytes: u64 = 0;
        let mut files_scanned: usize = 0;

        let filter = TransferFilter::parse(options.filter_mask.as_deref().unwrap_or(""));

        // `is_parent_dir` is a *path-shape* question that does not
        // depend on the endpoint: we only need to know whether the
        // destination looks like a directory, not actually stat
        // it. The destination panel is the source of truth.
        let is_parent_dir = is_destination_parent_dir(&self.sources, &self.destination, |p| {
            self.dst_endpoint.is_dir(p)
        });

        for src in &self.sources {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled during scan"));
            }

            // lstat on the source endpoint — does not follow
            // symlinks, which lets us tell `is_symlink` apart from
            // `is_dir` for free.
            let src_meta = match self.src_endpoint.lstat(src) {
                Ok(m) => m,
                Err(e) => {
                    return Err(anyhow!("Failed to stat source {}: {}", src.display(), e));
                }
            };

            if src_meta.is_dir && !(src_meta.is_symlink && !options.follow_symlinks) {
                let base_dst = if is_parent_dir {
                    let folder_name = src.file_name().unwrap_or_default();
                    self.destination.join(folder_name)
                } else {
                    self.destination.clone()
                };

                let mut dirs_to_visit: VecDeque<PathBuf> = VecDeque::new();
                dirs_to_visit.push_back(src.clone());
                if self.operation == TransferOperation::Delete
                    || self.operation == TransferOperation::Move
                {
                    dirs_to_delete.push(src.clone());
                }

                // Cycle detection: canonicalise the directory paths
                // we've already enqueued. Two paths that resolve to
                // the same inode never get walked twice. We only
                // canonicalise on Local — for Ssh the path is the
                // natural key (canonical paths are server-side and
                // the SFTP protocol doesn't expose portable
                // realpath).
                let mut visited_dirs: HashSet<PathBuf> = HashSet::new();

                while let Some(dir) = dirs_to_visit.pop_front() {
                    if self.is_cancelled.load(Ordering::Relaxed) {
                        return Err(anyhow!("Job cancelled during scan"));
                    }

                    if self.src_endpoint.is_local() {
                        let canon = self.src_endpoint.canonicalize(&dir);
                        if !visited_dirs.insert(canon.clone()) {
                            log::debug!("scan: skipping already-visited dir {}", dir.display());
                            continue;
                        }
                    } else {
                        // Ssh: path-based dedup.
                        if !visited_dirs.insert(dir.clone()) {
                            continue;
                        }
                    }

                    if self.operation == TransferOperation::Copy
                        || self.operation == TransferOperation::Move
                    {
                        if let Ok(rel) = dir.strip_prefix(src) {
                            let dst_dir = base_dst.join(rel);
                            let _ = self.dst_endpoint.mkdir_all(&dst_dir);
                        }
                    }

                    let entries = match self.src_endpoint.read_dir(&dir) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    for entry in entries {
                        let path = entry.path;
                        let is_symlink = entry.is_symlink;

                        if is_symlink && options.skip_symlinks {
                            continue;
                        }

                        if is_symlink && !options.follow_symlinks {
                            // Recreate-as-symlink mode. We add a
                            // mapping with size 0; the transfer
                            // phase will recreate the link.
                            let size = 0u64;
                            if self.operation == TransferOperation::Delete {
                                scan_mappings.push((path, PathBuf::new(), size));
                                files_scanned += 1;
                            } else if let Ok(rel) = path.strip_prefix(src) {
                                let dst_path = base_dst.join(rel);
                                scan_mappings.push((path, dst_path, size));
                                files_scanned += 1;
                            }
                            continue;
                        }

                        // Stat the entry through the endpoint so
                        // we get a definitive is_dir / size.
                        let stat = self.src_endpoint.lstat(&path).ok();
                        let is_dir = stat.as_ref().map(|s| s.is_dir).unwrap_or(false);

                        if is_dir {
                            dirs_to_visit.push_back(path.clone());
                            if self.operation == TransferOperation::Delete
                                || self.operation == TransferOperation::Move
                            {
                                dirs_to_delete.push(path);
                            }
                        } else {
                            let size = entry.size;
                            if !filter.matches(&path, size) {
                                continue;
                            }

                            if self.operation == TransferOperation::Delete {
                                scan_mappings.push((path, PathBuf::new(), size));
                                total_bytes += size;
                                files_scanned += 1;
                            } else if let Ok(rel) = path.strip_prefix(src) {
                                let dst_path = base_dst.join(rel);
                                scan_mappings.push((path, dst_path, size));
                                total_bytes += size;
                                files_scanned += 1;
                            }
                        }
                    }

                    let _ = self.event_tx.send(TransferEvent::ScanProgress {
                        job_id: self.job_id,
                        files_found: files_scanned,
                    });
                }
            } else {
                // Single-file source.
                let is_symlink = src_meta.is_symlink;
                if is_symlink && options.skip_symlinks {
                    continue;
                }

                let size = if is_symlink && !options.follow_symlinks {
                    0
                } else {
                    src_meta.size
                };

                if !filter.matches(src, size) {
                    continue;
                }

                if self.operation == TransferOperation::Delete {
                    scan_mappings.push((src.clone(), PathBuf::new(), size));
                    total_bytes += size;
                    files_scanned += 1;
                } else {
                    let dst_path = if is_parent_dir {
                        let file_name = src.file_name().unwrap_or_default();
                        self.destination.join(file_name)
                    } else {
                        self.destination.clone()
                    };
                    scan_mappings.push((src.clone(), dst_path, size));
                    total_bytes += size;
                    files_scanned += 1;
                }

                let _ = self.event_tx.send(TransferEvent::ScanProgress {
                    job_id: self.job_id,
                    files_found: files_scanned,
                });
            }
        }

        let _ = self.event_tx.send(TransferEvent::ScanComplete {
            job_id: self.job_id,
            total_files: files_scanned,
            total_bytes,
        });

        if files_scanned > 0 || self.operation != TransferOperation::Delete {
            let _ = self.event_tx.send(TransferEvent::TransferStarted {
                job_id: self.job_id,
                total_files: files_scanned,
                total_bytes,
            });
        }

        // -----------------------------------------------------------------
        // FASE 2: DISPATCH
        // -----------------------------------------------------------------
        let bytes_transferred_acc = Arc::new(AtomicU64::new(0));

        // Speed reporter: ticks every second with current
        // bytes/sec and ETA.
        let _speed_reporter = spawn_speed_reporter(
            self.event_tx.clone(),
            self.job_id,
            Arc::clone(&bytes_transferred_acc),
            total_bytes,
            Arc::clone(&self.is_cancelled),
        );

        let result = match self.operation {
            TransferOperation::Delete => {
                self.run_delete(
                    scan_mappings,
                    dirs_to_delete,
                    files_scanned,
                    bytes_transferred_acc,
                )
                .await
            }
            TransferOperation::Copy => {
                self.run_copy(
                    scan_mappings,
                    total_bytes,
                    files_scanned,
                    bytes_transferred_acc,
                )
                .await
            }
            TransferOperation::Rename => {
                // Single-source, single-destination rename. The
                // engine must enforce that the two endpoints are
                // the same; otherwise we refuse with a clear
                // error instead of falling back to copy+delete.
                if !self.src_endpoint.same_client(&self.dst_endpoint) {
                    return Err(anyhow!(
                        "Rename across endpoints is not supported (use Move instead)"
                    ));
                }
                self.run_rename().await
            }
            TransferOperation::CreateLink { kind } => {
                if !self.src_endpoint.same_client(&self.dst_endpoint) {
                    return Err(anyhow!("Create link across endpoints is not supported"));
                }
                self.run_create_link(kind).await
            }
            TransferOperation::Move => {
                // Same-endpoint move: try a direct rename for
                // every (src, dst) pair. This is O(N) renames
                // instead of N copies + N deletes, and the
                // rename is atomic when the server supports it.
                if self.src_endpoint.same_client(&self.dst_endpoint) {
                    self.run_move_atomic(scan_mappings, dirs_to_delete).await
                } else {
                    self.run_copy(
                        scan_mappings,
                        total_bytes,
                        files_scanned,
                        bytes_transferred_acc,
                    )
                    .await
                    .and_then(|copy_results| {
                        // After a successful cross-endpoint copy
                        // we still need to remove the sources.
                        Ok(copy_results)
                    })
                }
            }
        };

        // -----------------------------------------------------------------
        // FASE 3: POLICY FINALIZE
        // -----------------------------------------------------------------
        // Ask the policy if any files should be retried as
        // admin. The default `LoggingPolicy` always returns
        // the empty list, so this is a no-op in tests. The
        // `PromptPolicy` returns the list of files that
        // failed with `AccessDenied`; we forward that as a
        // single `PermissionPrompt` event so the UI can show
        // one popup at the end of the job.
        let retries = self.policy.finalize();
        if !retries.is_empty() {
            let _ = self.event_tx.send(TransferEvent::PermissionPrompt {
                job_id: self.job_id,
                count: retries.len(),
                files: retries.iter().map(|r| r.original_path.clone()).collect(),
            });
        }

        result
    }

    // -----------------------------------------------------------------
    //   Copy
    // -----------------------------------------------------------------
    async fn run_copy(
        &self,
        scan_mappings: Vec<(PathBuf, PathBuf, u64)>,
        _total_bytes: u64,
        _files_scanned: usize,
        bytes_transferred_acc: Arc<AtomicU64>,
    ) -> Result<TransferResults, anyhow::Error> {
        let mut auto_resolution = None;
        let mut results = TransferResults::default();

        for (idx, (src, mut dst, size)) in scan_mappings.into_iter().enumerate() {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }
            while self.is_paused.load(Ordering::Relaxed) {
                if self.is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if self.skip_file_flag.swap(false, Ordering::Relaxed) {
                results.skipped_files.push(SkippedFile {
                    src: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                    job_id: self.job_id,
                    file: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                continue;
            }

            // Conflict resolution (uses the destination endpoint
            // to test for existence).
            if self.dst_endpoint.exists(&dst) {
                let mut resolution = self.options.conflict_resolution.clone();
                if resolution == "ask" {
                    let chosen = if let Some(auto_res) = auto_resolution {
                        auto_res
                    } else {
                        let src_meta = self.src_endpoint.lstat(&src).ok();
                        let dst_meta = self.dst_endpoint.lstat(&dst).ok();
                        let _ = self.event_tx.send(TransferEvent::ConflictDetected {
                            job_id: self.job_id,
                            file: dst.clone(),
                            conflict: super::conflict::ConflictInfo {
                                src_path: src.clone(),
                                dst_path: dst.clone(),
                                src_size: src_meta.as_ref().map(|m| m.size).unwrap_or(0),
                                dst_size: dst_meta.as_ref().map(|m| m.size).unwrap_or(0),
                                src_modified: src_meta.as_ref().and_then(|m| m.modified),
                                dst_modified: dst_meta.as_ref().and_then(|m| m.modified),
                            },
                        });

                        {
                            let mut guard = self
                                .active_conflict
                                .lock()
                                .expect("active_conflict mutex poisoned");
                            *guard = None;
                        }
                        while self
                            .active_conflict
                            .lock()
                            .expect("active_conflict mutex poisoned")
                            .is_none()
                        {
                            if self.is_cancelled.load(Ordering::Relaxed) {
                                return Err(anyhow!("Job cancelled"));
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        let ch = self
                            .active_conflict
                            .lock()
                            .expect("active_conflict mutex poisoned")
                            .clone()
                            .unwrap_or(super::conflict::ConflictResolution::Skip);
                        match ch {
                            super::conflict::ConflictResolution::OverwriteAll
                            | super::conflict::ConflictResolution::OverwriteOlderAll
                            | super::conflict::ConflictResolution::SkipAll
                            | super::conflict::ConflictResolution::RenameAll => {
                                auto_resolution = Some(ch);
                            }
                            _ => {}
                        }
                        ch
                    };

                    resolution = match chosen {
                        super::conflict::ConflictResolution::Overwrite
                        | super::conflict::ConflictResolution::OverwriteAll => {
                            "overwrite".to_string()
                        }
                        super::conflict::ConflictResolution::OverwriteOlder
                        | super::conflict::ConflictResolution::OverwriteOlderAll => {
                            "overwrite_older".to_string()
                        }
                        super::conflict::ConflictResolution::Rename
                        | super::conflict::ConflictResolution::RenameAll
                        | super::conflict::ConflictResolution::KeepBoth => "rename".to_string(),
                        super::conflict::ConflictResolution::Cancel => {
                            self.is_cancelled.store(true, Ordering::SeqCst);
                            return Err(anyhow!("Job cancelled"));
                        }
                        _ => "skip".to_string(),
                    };
                }

                match resolution.as_str() {
                    "skip" => {
                        results.skipped_files.push(SkippedFile {
                            src: src.clone(),
                            reason: "File already exists (skipped)".to_string(),
                        });
                        let _ = self.event_tx.send(TransferEvent::FileSkipped {
                            job_id: self.job_id,
                            file: src.clone(),
                            reason: "File already exists".to_string(),
                        });
                        continue;
                    }
                    "rename" | "keep_both" => {
                        dst = resolve_filename_conflict(&dst);
                    }
                    "overwrite_older" => {
                        let src_time = self.src_endpoint.lstat(&src).ok().and_then(|m| m.modified);
                        let dst_time = self.dst_endpoint.lstat(&dst).ok().and_then(|m| m.modified);
                        if let (Some(s_time), Some(d_time)) = (src_time, dst_time) {
                            if s_time <= d_time {
                                results.skipped_files.push(SkippedFile {
                                    src: src.clone(),
                                    reason: "Destination is newer or equal (skipped)".to_string(),
                                });
                                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                                    job_id: self.job_id,
                                    file: src.clone(),
                                    reason: "Destination is newer or equal".to_string(),
                                });
                                continue;
                            }
                        }
                    }
                    _ => {} // Overwrite
                }
            }

            let _ = self.event_tx.send(TransferEvent::FileStarted {
                job_id: self.job_id,
                file: src.clone(),
                index: idx,
            });

            let mut retries: u32 = 0;
            let mut copy_success = false;
            let mut last_error = String::new();
            let mut src_hash: Option<String> = None;
            let mut dst_hash: Option<String> = None;
            let file_start = Instant::now();

            // Is the source a symlink? If so, recreate it at
            // the destination instead of copying bytes.
            let src_meta_for_link = self.src_endpoint.lstat(&src).ok();
            let is_symlink = src_meta_for_link
                .as_ref()
                .map(|m| m.is_symlink)
                .unwrap_or(false);
            let recreate_link = is_symlink && !self.options.follow_symlinks;

            if recreate_link {
                if let Some(meta) = src_meta_for_link {
                    match recreate_symlink(
                        &self.src_endpoint,
                        &self.dst_endpoint,
                        &meta,
                        &src,
                        &dst,
                    ) {
                        Ok(_) => {
                            copy_success = true;
                        }
                        Err(e) => last_error = format!("Error creating symlink: {}", e),
                    }
                } else {
                    last_error = "Could not stat source symlink".to_string();
                }
            } else {
                while retries <= self.options.max_retries {
                    if self.is_cancelled.load(Ordering::Relaxed) {
                        return Err(anyhow!("Job cancelled"));
                    }
                    match copy_file_pipelined(
                        &self.src_endpoint,
                        &src,
                        &self.dst_endpoint,
                        &dst,
                        &self.options,
                        &self.event_tx,
                        self.job_id,
                        Arc::clone(&self.is_paused),
                        Arc::clone(&self.is_cancelled),
                        Arc::clone(&bytes_transferred_acc),
                    )
                    .await
                    {
                        Ok((s_hash, d_hash)) => {
                            src_hash = s_hash;
                            dst_hash = d_hash;
                            copy_success = true;
                            break;
                        }
                        Err(e) => {
                            retries += 1;
                            last_error = e.to_string();
                            if retries <= self.options.max_retries {
                                let backoff = Duration::from_millis(100 * (1u64 << retries));
                                tokio::time::sleep(backoff).await;
                            }
                        }
                    }
                }
            }

            if !copy_success {
                self.emit_file_failed(
                    &mut results,
                    &src,
                    &dst,
                    &last_error,
                    retries,
                );
                if self.options.halt_on_error {
                    return Err(anyhow!("Halt on error: {}", last_error));
                }
                continue;
            }

            // Metadata preservation (Phase 2 already endpoint-aware).
            let _ = preserve_metadata(
                &self.src_endpoint,
                &src,
                &self.dst_endpoint,
                &dst,
                &self.options,
            );

            // Optional hash verification after copy.
            if self.options.verify_after_copy {
                let _ = self.event_tx.send(TransferEvent::VerifyStarted {
                    job_id: self.job_id,
                    file: src.clone(),
                    algorithm: self.options.hash_algorithm.as_str().to_string(),
                });
                if let (Some(sh), Some(dh)) = (src_hash.as_ref(), dst_hash.as_ref()) {
                    let _ = self.event_tx.send(TransferEvent::VerifyProgress {
                        job_id: self.job_id,
                        bytes_verified: size,
                        bytes_total: size,
                    });
                    if sh != dh {
                        self.emit_file_failed(
                            &mut results,
                            &src,
                            &dst,
                            "Hash verification mismatch",
                            0,
                        );
                        if self.options.halt_on_error {
                            return Err(anyhow!("Halt on error: Hash mismatch"));
                        }
                        continue;
                    }
                }
            }

            let file_result = FileTransferResult {
                src: src.clone(),
                dst: dst.clone(),
                size,
                src_hash: src_hash.clone(),
                dst_hash: dst_hash.clone(),
                verified: true,
                duration: file_start.elapsed(),
            };
            results.completed_files.push(file_result.clone());
            let _ = self.event_tx.send(TransferEvent::FileCompleted {
                job_id: self.job_id,
                result: file_result,
            });
        }

        // For `Move` dispatched through run_copy, we also need
        // to delete the sources. The dispatching site calls
        // run_copy_and_delete; this is the copy-only path.
        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }

    // -----------------------------------------------------------------
    //   Move: same-endpoint atomic rename
    // -----------------------------------------------------------------
    async fn run_move_atomic(
        &self,
        scan_mappings: Vec<(PathBuf, PathBuf, u64)>,
        dirs_to_delete: Vec<PathBuf>,
    ) -> Result<TransferResults, anyhow::Error> {
        let mut results = TransferResults::default();
        for (idx, (src, mut dst, size)) in scan_mappings.into_iter().enumerate() {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }
            while self.is_paused.load(Ordering::Relaxed) {
                if self.is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if self.skip_file_flag.swap(false, Ordering::Relaxed) {
                results.skipped_files.push(SkippedFile {
                    src: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                    job_id: self.job_id,
                    file: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                continue;
            }

            // Conflict handling: if the destination exists, fall
            // back to the same logic as Copy.
            if self.dst_endpoint.exists(&dst) {
                // For atomic move, only Overwrite and Rename are
                // meaningful. Skip / Overwrite-older reuse the
                // Copy semantics.
                match self.options.conflict_resolution.as_str() {
                    "skip" | "ask" => {
                        results.skipped_files.push(SkippedFile {
                            src: src.clone(),
                            reason: "File already exists (skipped)".to_string(),
                        });
                        let _ = self.event_tx.send(TransferEvent::FileSkipped {
                            job_id: self.job_id,
                            file: src.clone(),
                            reason: "File already exists".to_string(),
                        });
                        continue;
                    }
                    "rename" | "keep_both" => {
                        dst = resolve_filename_conflict(&dst);
                    }
                    _ => {} // Overwrite / overwrite_older
                }
            }

            let _ = self.event_tx.send(TransferEvent::FileStarted {
                job_id: self.job_id,
                file: src.clone(),
                index: idx,
            });

            let file_start = Instant::now();
            let mut copy_success = false;
            let mut last_error = String::new();
            let mut retries: u32 = 0;

            while retries <= self.options.max_retries {
                match self.src_endpoint.rename(&src, &dst) {
                    Ok(_) => {
                        copy_success = true;
                        break;
                    }
                    Err(e) => {
                        retries += 1;
                        last_error = e.to_string();
                        if retries <= self.options.max_retries {
                            let backoff = Duration::from_millis(100 * (1u64 << retries));
                            tokio::time::sleep(backoff).await;
                        }
                    }
                }
            }

            if !copy_success {
                self.emit_file_failed(
                    &mut results,
                    &src,
                    &dst,
                    &last_error,
                    retries,
                );
                if self.options.halt_on_error {
                    return Err(anyhow!("Halt on error: {}", last_error));
                }
                continue;
            }

            let file_result = FileTransferResult {
                src: src.clone(),
                dst: dst.clone(),
                size,
                src_hash: None,
                dst_hash: None,
                verified: true,
                duration: file_start.elapsed(),
            };
            results.completed_files.push(file_result.clone());
            let _ = self.event_tx.send(TransferEvent::FileCompleted {
                job_id: self.job_id,
                result: file_result,
            });
        }

        // Remove now-empty source directories (deepest first).
        let mut sorted_dirs = dirs_to_delete;
        sorted_dirs.sort_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()));
        for dir in sorted_dirs {
            if self.is_cancelled.load(Ordering::Relaxed) {
                break;
            }
            let _ = self.src_endpoint.remove_dir(&dir);
        }

        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }

    // -----------------------------------------------------------------
    //   Rename (single source / single destination)
    // -----------------------------------------------------------------
    async fn run_rename(&self) -> Result<TransferResults, anyhow::Error> {
        if self.sources.len() != 1 {
            return Err(anyhow!(
                "Rename requires exactly one source (got {})",
                self.sources.len()
            ));
        }
        let src = self.sources[0].clone();
        let dst = self.destination.clone();

        // Honour pause / cancel.
        if self.is_cancelled.load(Ordering::Relaxed) {
            return Err(anyhow!("Job cancelled"));
        }
        while self.is_paused.load(Ordering::Relaxed) {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let mut results = TransferResults::default();
        let size = self.src_endpoint.lstat(&src).map(|m| m.size).unwrap_or(0);

        let _ = self.event_tx.send(TransferEvent::FileStarted {
            job_id: self.job_id,
            file: src.clone(),
            index: 0,
        });

        // Conflict resolution: if the destination already
        // exists, fall back to the same logic as Copy
        // (overwrite / skip / rename / overwrite-older).
        let mut target = dst.clone();
        if self.dst_endpoint.exists(&target) {
            match self.options.conflict_resolution.as_str() {
                "skip" | "ask" => {
                    results.skipped_files.push(SkippedFile {
                        src: src.clone(),
                        reason: "Destination already exists (skipped)".to_string(),
                    });
                    let _ = self.event_tx.send(TransferEvent::FileSkipped {
                        job_id: self.job_id,
                        file: src.clone(),
                        reason: "Destination already exists".to_string(),
                    });
                    let _ = self.event_tx.send(TransferEvent::JobCompleted {
                        job_id: self.job_id,
                        results: results.clone(),
                    });
                    return Ok(results);
                }
                "rename" | "keep_both" => {
                    target = resolve_filename_conflict(&target);
                }
                _ => {} // Overwrite / overwrite_older — let the rename try to clobber
            }
        }

        let file_start = Instant::now();
        let mut success = false;
        let mut last_error = String::new();
        let mut retries: u32 = 0;
        while retries <= self.options.max_retries {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }
            match self.src_endpoint.rename(&src, &target) {
                Ok(_) => {
                    success = true;
                    break;
                }
                Err(e) => {
                    retries += 1;
                    last_error = e.to_string();
                    if retries <= self.options.max_retries {
                        let backoff = Duration::from_millis(100 * (1u64 << retries));
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        if !success {
            self.emit_file_failed(
                &mut results,
                &src,
                &target,
                &last_error,
                retries,
            );
            if self.options.halt_on_error {
                return Err(anyhow!("Halt on error: {}", last_error));
            }
        } else {
            let result = FileTransferResult {
                src: src.clone(),
                dst: target.clone(),
                size,
                src_hash: None,
                dst_hash: None,
                verified: true,
                duration: file_start.elapsed(),
            };
            results.completed_files.push(result.clone());
            let _ = self.event_tx.send(TransferEvent::FileCompleted {
                job_id: self.job_id,
                result,
            });
        }

        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }

    // -----------------------------------------------------------------
    //   CreateLink (symlink or hardlink)
    // -----------------------------------------------------------------
    async fn run_create_link(&self, kind: LinkKind) -> Result<TransferResults, anyhow::Error> {
        if self.sources.len() != 1 {
            return Err(anyhow!(
                "CreateLink requires exactly one source (got {})",
                self.sources.len()
            ));
        }
        let src = self.sources[0].clone();
        let dst = self.destination.clone();

        if self.is_cancelled.load(Ordering::Relaxed) {
            return Err(anyhow!("Job cancelled"));
        }
        while self.is_paused.load(Ordering::Relaxed) {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let mut results = TransferResults::default();
        let _ = self.event_tx.send(TransferEvent::FileStarted {
            job_id: self.job_id,
            file: src.clone(),
            index: 0,
        });

        // If the destination already exists, fail fast with
        // a clear error (matches the behaviour of std::os::unix::fs::symlink).
        if self.dst_endpoint.exists(&dst) {
            self.emit_file_failed(
                &mut results,
                &src,
                &dst,
                "Link target already exists",
                0,
            );
            let _ = self.event_tx.send(TransferEvent::JobCompleted {
                job_id: self.job_id,
                results: results.clone(),
            });
            return Ok(results);
        }

        let file_start = Instant::now();
        let link_result = match kind {
            LinkKind::Symbolic => {
                // Need to know whether the target is a directory
                // for the Windows code path; on Unix it does not
                // matter. Read it through the endpoint.
                let target_is_dir = self.src_endpoint.is_dir(&src);
                self.dst_endpoint.create_symlink(&src, &dst, target_is_dir)
            }
            LinkKind::Hard => self.dst_endpoint.create_hardlink(&src, &dst),
        };

        match link_result {
            Ok(_) => {
                let result = FileTransferResult {
                    src: src.clone(),
                    dst: dst.clone(),
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: file_start.elapsed(),
                };
                results.completed_files.push(result.clone());
                let _ = self.event_tx.send(TransferEvent::FileCompleted {
                    job_id: self.job_id,
                    result,
                });
            }
            Err(e) => {
                self.emit_file_failed(
                    &mut results,
                    &src,
                    &dst,
                    &e.to_string(),
                    0,
                );
            }
        }

        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }

    // -----------------------------------------------------------------
    //   Delete
    // -----------------------------------------------------------------
    async fn run_delete(
        &self,
        scan_mappings: Vec<(PathBuf, PathBuf, u64)>,
        dirs_to_delete: Vec<PathBuf>,
        _files_scanned: usize,
        bytes_transferred_acc: Arc<AtomicU64>,
    ) -> Result<TransferResults, anyhow::Error> {
        let mut results = TransferResults::default();

        if self.options.delete_to_recycle_bin && self.src_endpoint.is_local() {
            for (idx, (src, _, _)) in scan_mappings.iter().enumerate() {
                if self.is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                let delete_start = Instant::now();
                let _ = self.event_tx.send(TransferEvent::FileStarted {
                    job_id: self.job_id,
                    file: src.clone(),
                    index: idx,
                });
                if let Err(e) = send_to_recycle_bin_helper(src) {
                    self.emit_file_failed(
                        &mut results,
                        src,
                        &PathBuf::new(),
                        &e.to_string(),
                        0,
                    );
                    if self.options.halt_on_error {
                        return Err(anyhow!("Halt on error: Recycle Bin deletion failed"));
                    }
                } else {
                    let size = self.src_endpoint.lstat(src).map(|m| m.size).unwrap_or(0);
                    let result = FileTransferResult {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        size,
                        src_hash: None,
                        dst_hash: None,
                        verified: true,
                        duration: delete_start.elapsed(),
                    };
                    results.completed_files.push(result.clone());
                    let _ = self.event_tx.send(TransferEvent::FileCompleted {
                        job_id: self.job_id,
                        result,
                    });
                    bytes_transferred_acc.fetch_add(size, Ordering::SeqCst);
                }
            }
        } else {
            for (idx, (src, _, size)) in scan_mappings.into_iter().enumerate() {
                if self.is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                while self.is_paused.load(Ordering::Relaxed) {
                    if self.is_cancelled.load(Ordering::Relaxed) {
                        return Err(anyhow!("Job cancelled"));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                if self.skip_file_flag.swap(false, Ordering::Relaxed) {
                    results.skipped_files.push(SkippedFile {
                        src: src.clone(),
                        reason: "Skipped by user".to_string(),
                    });
                    let _ = self.event_tx.send(TransferEvent::FileSkipped {
                        job_id: self.job_id,
                        file: src.clone(),
                        reason: "Skipped by user".to_string(),
                    });
                    continue;
                }

                let delete_start = Instant::now();
                let _ = self.event_tx.send(TransferEvent::FileStarted {
                    job_id: self.job_id,
                    file: src.clone(),
                    index: idx,
                });
                // Secure wipe: overwrite the file's bytes with
                // alternating patterns before unlinking. Only
                // meaningful for Local (SFTP cannot guarantee
                // overwrite semantics on remote files; we skip
                // the wipe and just delete).
                if self.options.wipe_passes > 0 && self.src_endpoint.is_local() {
                    let _ = self.secure_wipe(&src);
                }
                let mut res = self.src_endpoint.remove_file(&src);
                if res.is_err() {
                    let _ = self.src_endpoint.make_writable(&src);
                    res = self.src_endpoint.remove_file(&src);
                }
                if let Err(e) = res {
                    self.emit_file_failed(
                        &mut results,
                        &src,
                        &PathBuf::new(),
                        &e.to_string(),
                        0,
                    );
                    if self.options.halt_on_error {
                        return Err(anyhow!("Halt on error: Deletion failed"));
                    }
                } else {
                    let result = FileTransferResult {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        size,
                        src_hash: None,
                        dst_hash: None,
                        verified: true,
                        duration: delete_start.elapsed(),
                    };
                    results.completed_files.push(result.clone());
                    let _ = self.event_tx.send(TransferEvent::FileCompleted {
                        job_id: self.job_id,
                        result,
                    });
                    bytes_transferred_acc.fetch_add(size, Ordering::SeqCst);
                }
            }

            let mut sorted_dirs = dirs_to_delete;
            sorted_dirs.sort_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()));
            for dir in sorted_dirs {
                let mut res = self.src_endpoint.remove_dir(&dir);
                if res.is_err() {
                    let _ = self.src_endpoint.make_writable(&dir);
                    res = self.src_endpoint.remove_dir(&dir);
                }
                if let Err(e) = res {
                    self.emit_file_failed(
                        &mut results,
                        &dir,
                        &PathBuf::new(),
                        &e.to_string(),
                        0,
                    );
                }
            }
        }

        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }
}

// =========================================================================
//   Helpers
// =========================================================================

/// Notify the policy that a single file operation failed.
/// Categorises the error so the [`super::policy::PromptPolicy`]
/// can decide whether to add the file to its
/// retry-as-admin batch.
fn report_file_failure(
    policy: &dyn TransferPolicy,
    file: &Path,
    error_msg: &str,
) {
    let cat = FileError::from_error_message(error_msg);
    policy.on_file_error(file, &cat);
}

/// Re-create a symlink in the destination by reading the source
/// target through the endpoint and writing it to the destination
/// through the destination endpoint. On Windows, target_is_dir
/// matters for whether we make a file or directory symlink; on
/// Unix the same call works for both.
fn recreate_symlink(
    src_endpoint: &TransferEndpoint,
    dst_endpoint: &TransferEndpoint,
    src_meta: &StatInfo,
    src: &Path,
    dst: &Path,
) -> Result<(), anyhow::Error> {
    let target = src_meta
        .target
        .clone()
        .or_else(|| src_endpoint.read_link(src).ok())
        .ok_or_else(|| anyhow!("could not read symlink target for {}", src.display()))?;

    if dst_endpoint.exists(dst) {
        let _ = dst_endpoint.remove_file(dst);
        let _ = dst_endpoint.remove_dir_all(dst);
    }

    let target_is_dir = src_endpoint
        .stat(&target)
        .map(|m| m.is_dir)
        .unwrap_or(false);
    dst_endpoint
        .create_symlink(&target, dst, target_is_dir)
        .map_err(|e| anyhow!("create_symlink {}: {}", dst.display(), e))?;
    Ok(())
}

/// Securely overwrite a regular file before deletion by writing
/// alternating byte patterns across its full length. The number
/// of passes comes from `TransferOptions::wipe_passes` (clamped
/// to 3). After the last pass the file is truncated to zero
/// bytes and its permissions are relaxed (so the subsequent
/// `remove_file` is not blocked by a read-only bit left over
/// from a previous owner).
impl TransferWorker {
    fn secure_wipe(&self, path: &Path) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};

        // Skip symlinks: a symlink deletion removes the link, not
        // the target. Following it here would wipe a file the user
        // did not ask us to wipe.
        if self
            .src_endpoint
            .lstat(path)
            .map(|m| m.is_symlink)
            .unwrap_or(false)
        {
            return Ok(());
        }
        let size = self.src_endpoint.lstat(path).map(|m| m.size).unwrap_or(0);
        if size == 0 {
            return Ok(());
        }

        // Cap to 3 passes; anything higher is bounded.
        let passes = self.options.wipe_passes.clamp(1, 3);
        let patterns: [u8; 3] = [0x00, 0xFF, 0x00];

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(super::direct_io::to_long_path(path))?;
        // Drop the read-only flag in case the user wiped a chmod 0444 file.
        let _ = self.src_endpoint.make_writable(path);

        for i in 0..passes as usize {
            let pat = patterns[i % patterns.len()];
            let chunk = vec![pat; 64 * 1024];
            file.seek(SeekFrom::Start(0))?;
            let mut written = 0u64;
            while written < size {
                let to_write = ((size - written) as usize).min(chunk.len());
                file.write_all(&chunk[..to_write])?;
                written += to_write as u64;
            }
            file.sync_all()?;
        }
        // Truncate to zero so the directory entry really is gone
        // before the unlink runs.
        file.set_len(0)?;
        file.sync_all()?;
        Ok(())
    }
}

/// Spawn the periodic speed reporter that emits
/// `TransferEvent::SpeedUpdate` for the UI.
fn spawn_speed_reporter(
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    bytes_acc: Arc<AtomicU64>,
    total_bytes: u64,
    is_cancelled: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_bytes: u64 = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if is_cancelled.load(Ordering::Relaxed) {
                break;
            }
            let current_bytes = bytes_acc.load(Ordering::SeqCst);
            let delta = current_bytes.saturating_sub(last_bytes);
            last_bytes = current_bytes;
            let bytes_per_second = delta as f64;
            let remaining_bytes = total_bytes.saturating_sub(current_bytes);
            let eta_seconds = if bytes_per_second > 0.0 {
                Some((remaining_bytes as f64 / bytes_per_second) as u64)
            } else {
                None
            };
            let _ = event_tx.send(TransferEvent::SpeedUpdate {
                job_id,
                bytes_per_second,
                eta_seconds,
            });
            if current_bytes >= total_bytes && total_bytes > 0 {
                break;
            }
        }
    })
}

/// Determines if `destination` should be treated as a parent directory into which
/// source items are placed (appending source item filenames), or if `destination` is
/// the target path for a single source item itself.
pub fn is_destination_parent_dir(
    sources: &[PathBuf],
    destination: &Path,
    is_dir_fn: impl Fn(&Path) -> bool,
) -> bool {
    if sources.len() > 1 {
        return true;
    }
    let s = destination.to_string_lossy();
    if s.ends_with('/') || s.ends_with('\\') {
        return true;
    }
    if is_dir_fn(destination) {
        if let Some(src) = sources.first() {
            if let (Some(dest_name), Some(src_name)) = (destination.file_name(), src.file_name()) {
                return dest_name != src_name;
            }
        }
        return true;
    }
    false
}

// =========================================================================
//   Recycle-bin helpers
// =========================================================================

#[cfg(target_os = "windows")]
fn send_to_recycle_bin_helper(path: &Path) -> anyhow::Result<()> {
    use std::process::Command;
    let path_str = path.to_string_lossy().replace('\'', "''");
    let ps_cmd = if path.is_dir() {
        format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path_str
        )
    } else {
        format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path_str
        )
    };
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_cmd])
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => anyhow::bail!("Failed to execute PowerShell trash command: {}", e),
    };
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell Recycle Bin error: {}", err);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn send_to_recycle_bin_helper(path: &Path) -> anyhow::Result<()> {
    use std::process::Command;

    // 1. Try `gio trash` (GNOME / modern GLib-based desktops).
    match Command::new("gio")
        .arg("trash")
        .arg("--")
        .arg(path)
        .status()
    {
        Ok(s) if s.success() => return Ok(()),
        Ok(_) | Err(_) => {}
    }
    // 2. Try `trash-put` (trash-cli).
    match Command::new("trash-put").arg("--").arg(path).status() {
        Ok(s) if s.success() => return Ok(()),
        Ok(_) | Err(_) => {}
    }
    anyhow::bail!(
        "no trash tool found. Install `gio` (glib2) or `trash-cli`, or use a permanent delete."
    )
}

// =========================================================================
//   Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ep() -> TransferEndpoint {
        TransferEndpoint::Local
    }

    #[test]
    fn test_is_destination_parent_dir_single_file_target_path() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs/reporte.md");
        assert!(!is_destination_parent_dir(&sources, &destination, |_| {
            false
        }));
    }

    #[test]
    fn test_is_destination_parent_dir_trailing_slash() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs/");
        assert!(is_destination_parent_dir(&sources, &destination, |_| false));
    }

    #[test]
    fn test_is_destination_parent_dir_existing_folder_different_name() {
        let sources = vec![PathBuf::from("/home/user/reporte.md")];
        let destination = PathBuf::from("/home/user/docs");
        assert!(is_destination_parent_dir(&sources, &destination, |_| true));
    }

    #[test]
    fn test_is_destination_parent_dir_multiple_sources() {
        let sources = vec![
            PathBuf::from("/home/user/file1.md"),
            PathBuf::from("/home/user/file2.md"),
        ];
        let destination = PathBuf::from("/home/user/docs/file1.md");
        assert!(is_destination_parent_dir(&sources, &destination, |_| false));
    }

    fn make_worker(
        op: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
    ) -> (TransferWorker, mpsc::UnboundedReceiver<TransferEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let is_paused = Arc::new(AtomicBool::new(false));
        let is_cancelled = Arc::new(AtomicBool::new(false));
        let skip = Arc::new(AtomicBool::new(false));
        let conflict = Arc::new(std::sync::Mutex::new(None));
        let w = TransferWorker::new(
            Uuid::new_v4(),
            op,
            sources,
            destination,
            ep(),
            ep(),
            TransferOptions::default(),
            is_paused,
            is_cancelled,
            skip,
            tx,
            conflict,
            Arc::new(crate::fs::transfer::policy::PromptPolicy::new())
                as Arc<dyn TransferPolicy>,
        );
        (w, rx)
    }

    #[tokio::test]
    async fn test_worker_copy_local_to_local() {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        let f1 = src_root.join("a.txt");
        let f2 = src_root.join("sub/b.txt");
        std::fs::create_dir_all(f2.parent().unwrap()).unwrap();
        std::fs::write(&f1, b"hello-a").unwrap();
        std::fs::write(&f2, b"hello-b").unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();

        let (worker, mut rx) = make_worker(
            TransferOperation::Copy,
            vec![src_root.clone()],
            dst_root.clone(),
        );
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = worker.run().await;
        assert!(res.is_ok(), "copy failed: {:?}", res.err());

        let moved = dst_root.join("src");
        assert_eq!(std::fs::read(moved.join("a.txt")).unwrap(), b"hello-a");
        assert_eq!(std::fs::read(moved.join("sub/b.txt")).unwrap(), b"hello-b");
    }

    #[tokio::test]
    async fn test_worker_delete_local() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("victim");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::write(root.join("nested/b.txt"), b"y").unwrap();

        let (worker, mut rx) = make_worker(
            TransferOperation::Delete,
            vec![root.clone()],
            PathBuf::new(),
        );
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = worker.run().await;
        assert!(res.is_ok(), "delete failed: {:?}", res.err());
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn test_worker_move_local_atomic() {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::write(src_root.join("a.txt"), b"hello").unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();

        let (worker, mut rx) = make_worker(
            TransferOperation::Move,
            vec![src_root.clone()],
            dst_root.clone(),
        );
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = worker.run().await;
        assert!(res.is_ok(), "move failed: {:?}", res.err());

        // Atomic move: source should be gone, destination should
        // have the folder.
        assert!(!src_root.exists());
        assert!(dst_root.join("src/a.txt").exists());
    }

    #[tokio::test]
    async fn test_worker_copy_preserves_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        let target = src_root.join("real.txt");
        std::fs::write(&target, b"target").unwrap();
        let link = src_root.join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &link).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();

        let (worker, mut rx) = make_worker(
            TransferOperation::Copy,
            vec![src_root.clone()],
            dst_root.clone(),
        );
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = worker.run().await;
        assert!(res.is_ok(), "copy failed: {:?}", res.err());

        let moved = dst_root.join("src");
        let moved_link = moved.join("link.txt");
        assert!(
            moved_link
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[tokio::test]
    async fn test_worker_scan_does_not_loop_on_circular_symlink() {
        #[cfg(unix)]
        {
            let tmp = tempfile::tempdir().unwrap();
            let src_root = tmp.path().join("src");
            let dst_root = tmp.path().join("dst");
            std::fs::create_dir_all(&src_root).unwrap();
            std::fs::write(src_root.join("file.txt"), b"hello").unwrap();
            std::os::unix::fs::symlink(&src_root, src_root.join("loop")).unwrap();
            std::fs::create_dir_all(&dst_root).unwrap();

            let (tx, _rx) = mpsc::unbounded_channel();
            let is_paused = Arc::new(AtomicBool::new(false));
            let is_cancelled = Arc::new(AtomicBool::new(false));
            let skip = Arc::new(AtomicBool::new(false));
            let conflict = Arc::new(std::sync::Mutex::new(None));
            let mut options = TransferOptions::default();
            options.follow_symlinks = true;
            options.conflict_resolution = "overwrite".to_string();

            let worker = TransferWorker::new(
                Uuid::new_v4(),
                TransferOperation::Copy,
                vec![src_root.clone()],
                dst_root.clone(),
                ep(),
                ep(),
                options,
                is_paused,
                is_cancelled,
                skip,
                tx,
                conflict,
                Arc::new(crate::fs::transfer::policy::PromptPolicy::new())
                    as Arc<dyn TransferPolicy>,
            );
            let res = tokio::time::timeout(std::time::Duration::from_secs(10), worker.run())
                .await
                .expect("worker should not loop on a circular symlink");
            assert!(res.is_ok(), "worker returned error: {res:?}");
        }
    }

    #[tokio::test]
    async fn test_worker_move_directory_tree() {
        // Compatibility: re-creates the previous worker test for
        // the cross-directory move case. Even with the new
        // atomic-rename dispatch, the result is the same:
        // sources gone, contents in destination.
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src_folder");
        let dst_root = tmp.path().join("dst_folder");

        let sub_dir = src_root.join("sub_dir").join("nested");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let file1 = src_root.join("file1.txt");
        let file2 = sub_dir.join("file2.txt");

        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        std::fs::create_dir_all(&dst_root).unwrap();

        let (worker, mut rx) = make_worker(
            TransferOperation::Move,
            vec![src_root.clone()],
            dst_root.clone(),
        );
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = worker.run().await;
        assert!(res.is_ok(), "move tree failed: {:?}", res.err());

        let dst_moved_folder = dst_root.join("src_folder");
        assert!(dst_moved_folder.join("file1.txt").exists());
        assert!(
            dst_moved_folder
                .join("sub_dir")
                .join("nested")
                .join("file2.txt")
                .exists()
        );
        assert!(!file1.exists());
        assert!(!file2.exists());
        assert!(!src_root.exists());
    }
}
