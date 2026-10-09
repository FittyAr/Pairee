use anyhow::anyhow;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::super::control::JobControl;
use super::super::events::TransferEvent;
use super::super::filter::TransferFilter;
use super::super::job::TransferOperation;
use super::super::options::TransferOptions;
use super::destination::{ensure_dir, is_destination_parent_dir};

/// Result of the scan phase: file mappings and aggregate totals.
pub(super) struct ScanOutcome {
    pub mappings: Vec<(PathBuf, PathBuf, u64)>,
    pub dirs_to_delete: Vec<PathBuf>,
    /// Destination folders created while scanning.
    pub created_dirs: Vec<PathBuf>,
    pub total_bytes: u64,
    // Part of phase outcome API; ScanComplete is already emitted during scan.
    #[allow(dead_code)]
    pub files_scanned: usize,
}

/// Scan sources and build destination mappings for the transfer.
pub(super) fn scan(
    sources: &[PathBuf],
    destination: &Path,
    operation: TransferOperation,
    options: &TransferOptions,
    ctl: &JobControl,
) -> Result<ScanOutcome, anyhow::Error> {
    ctl.emit(TransferEvent::ScanProgress {
        job_id: ctl.job_id,
        files_found: 0,
    });

    let mut scanner = Scanner {
        operation,
        options,
        filter: TransferFilter::parse(options.filter_mask.as_deref().unwrap_or("")),
        ctl,
        out: ScanOutcome {
            mappings: Vec::new(),
            dirs_to_delete: Vec::new(),
            created_dirs: Vec::new(),
            total_bytes: 0,
            files_scanned: 0,
        },
    };
    let is_parent_dir = is_destination_parent_dir(sources, destination, |p| p.is_dir());
    // Inside the destination folder, or the destination itself.
    let target = |src: &Path| {
        if is_parent_dir {
            destination.join(src.file_name().unwrap_or_default())
        } else {
            destination.to_path_buf()
        }
    };

    for src in sources {
        if ctl.is_cancelled() {
            return Err(anyhow!("Job cancelled during scan"));
        }
        if src.is_dir() && (!src.is_symlink() || options.follow_symlinks) {
            scanner.scan_tree(src, &target(src))?;
        } else {
            scanner.scan_single(src, target(src));
        }
    }

    let out = scanner.out;
    ctl.emit(TransferEvent::ScanComplete {
        job_id: ctl.job_id,
        total_files: out.files_scanned,
        total_bytes: out.total_bytes,
    });
    Ok(out)
}

/// Walks the sources of one job, collecting mappings and totals.
struct Scanner<'a> {
    operation: TransferOperation,
    options: &'a TransferOptions,
    filter: TransferFilter,
    ctl: &'a JobControl,
    out: ScanOutcome,
}

impl Scanner<'_> {
    fn is_delete(&self) -> bool {
        self.operation == TransferOperation::Delete
    }

    /// Delete and move remove the scanned folders afterwards.
    fn removes_dirs(&self) -> bool {
        matches!(
            self.operation,
            TransferOperation::Delete | TransferOperation::Move
        )
    }

    fn push(&mut self, src: PathBuf, dst: PathBuf, size: u64) {
        self.out.mappings.push((src, dst, size));
        self.out.total_bytes += size;
        self.out.files_scanned += 1;
    }

    fn progress(&self) {
        self.ctl.emit(TransferEvent::ScanProgress {
            job_id: self.ctl.job_id,
            files_found: self.out.files_scanned,
        });
    }

    /// Destination of `path` found under the source folder `src` (none for a delete).
    fn dst_in_tree(&self, path: &Path, src: &Path, base_dst: &Path) -> Option<PathBuf> {
        if self.is_delete() {
            return Some(PathBuf::new());
        }
        path.strip_prefix(src).ok().map(|rel| base_dst.join(rel))
    }

    /// A source file (or a symlink that is not followed).
    fn scan_single(&mut self, src: &Path, dst: PathBuf) {
        let is_symlink = src.is_symlink();
        if is_symlink && self.options.skip_symlinks {
            return;
        }
        let size = if is_symlink && !self.options.follow_symlinks {
            0
        } else {
            src.metadata().map(|m| m.len()).unwrap_or(0)
        };
        if !self.filter.matches(src, size) {
            return;
        }
        let dst = if self.is_delete() {
            PathBuf::new()
        } else {
            dst
        };
        self.push(src.to_path_buf(), dst, size);
        self.progress();
    }

    /// A source folder, breadth first; destination folders are created for copy and move.
    fn scan_tree(&mut self, src: &Path, base_dst: &Path) -> Result<(), anyhow::Error> {
        let mut dirs_to_visit = VecDeque::from([src.to_path_buf()]);
        if self.removes_dirs() {
            self.out.dirs_to_delete.push(src.to_path_buf());
        }
        while let Some(dir) = dirs_to_visit.pop_front() {
            if self.ctl.is_cancelled() {
                return Err(anyhow!("Job cancelled during scan"));
            }
            if matches!(
                self.operation,
                TransferOperation::Copy | TransferOperation::Move
            ) && let Ok(rel) = dir.strip_prefix(src)
            {
                ensure_dir(&base_dst.join(rel), &mut self.out.created_dirs);
            }
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if let Some(subdir) = self.scan_entry(&entry, src, base_dst) {
                    dirs_to_visit.push_back(subdir);
                }
            }
            self.progress();
        }
        Ok(())
    }

    /// One folder entry; returns a subfolder still to visit.
    fn scan_entry(
        &mut self,
        entry: &std::fs::DirEntry,
        src: &Path,
        base_dst: &Path,
    ) -> Option<PathBuf> {
        let path = entry.path();
        let is_symlink = path.is_symlink();
        if is_symlink && self.options.skip_symlinks {
            return None;
        }
        if is_symlink && !self.options.follow_symlinks {
            if let Some(dst) = self.dst_in_tree(&path, src, base_dst) {
                self.push(path, dst, 0);
            }
            return None;
        }
        if path.is_dir() {
            if self.filter.excludes(&entry.file_name().to_string_lossy()) {
                return None;
            }
            if self.removes_dirs() {
                self.out.dirs_to_delete.push(path.clone());
            }
            return Some(path);
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if self.filter.matches(&path, size)
            && let Some(dst) = self.dst_in_tree(&path, src, base_dst)
        {
            self.push(path, dst, size);
        }
        None
    }
}

/// Scan of explicit `(source, destination)` file pairs: missing sources are
/// left out and the destination folders are created.
pub(super) fn scan_pairs(pairs: &[(PathBuf, PathBuf)], ctl: &JobControl) -> ScanOutcome {
    let mut created_dirs = Vec::new();
    let mappings: Vec<(PathBuf, PathBuf, u64)> = pairs
        .iter()
        .filter_map(|(from, to)| {
            let meta = from.symlink_metadata().ok()?;
            if let Some(parent) = to.parent() {
                ensure_dir(parent, &mut created_dirs);
            }
            let size = if meta.file_type().is_symlink() {
                0
            } else {
                meta.len()
            };
            Some((from.clone(), to.clone(), size))
        })
        .collect();
    let total_bytes = mappings.iter().map(|(_, _, size)| size).sum();
    ctl.scan_complete(mappings.len(), total_bytes);
    ScanOutcome {
        files_scanned: mappings.len(),
        mappings,
        dirs_to_delete: Vec::new(),
        created_dirs,
        total_bytes,
    }
}
