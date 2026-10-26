//! Scan phase shared by the local worker and the remote backend: walks the
//! sources (through the shared [`Walker`]) and lists what to transfer.
//!
//! * Files become `(source, destination, size)` mappings; folders become
//!   destination folders to create (copy, move) and source folders to remove
//!   afterwards (move, delete).
//! * Links follow the job options: skipped, followed (loops reported) or
//!   kept. A kept link is recreated where the destination supports links,
//!   copied as the file it points to elsewhere, and a link to a folder that
//!   cannot be recreated is skipped with a warning.
//! * Nothing is swallowed: unreadable folders, loops and sources that cannot
//!   be copied (a folder into itself) are returned as refused entries.
//! * A delete never follows links, so it can never leave the tree.

pub mod guard;
mod pairs;

#[cfg(test)]
mod tests;

pub use pairs::scan_pairs;

use super::control::{JobControl, failed, failed_io, walk_failed};
use super::filter::TransferFilter;
use super::job::DestinationKind;
use super::job::{FailedFile, SkippedFile, TransferOperation};
use super::options::TransferOptions;
use super::worker::is_destination_parent_dir;
use crate::config::localization::t;
use crate::fs::vfs::{Vfs, VfsEntry};
use crate::fs::walk::{LinkPolicy, WalkEntry, WalkError, WalkOptions, Walker};
use std::path::{Component, Path, PathBuf};

/// One file to transfer: source, destination (empty for a delete), size.
pub type FileMapping = (PathBuf, PathBuf, u64);

/// What one job scans, and the filesystems it reads and writes.
pub struct ScanRequest<'a> {
    /// Each source with the path it is transferred to ([`top_targets`]).
    pub roots: &'a [(PathBuf, PathBuf)],
    pub operation: TransferOperation,
    pub options: &'a TransferOptions,
    pub src_fs: &'a dyn Vfs,
    pub dst_fs: &'a dyn Vfs,
    /// Source and destination are one filesystem (the copy-into-itself
    /// guard applies).
    pub same_fs: bool,
    /// Links can be recreated at the destination.
    pub dst_links: bool,
}

/// Result of the scan phase.
#[derive(Debug, Default)]
pub struct ScanOutcome {
    pub mappings: Vec<FileMapping>,
    /// Destination folders to create, parents first.
    pub dst_dirs: Vec<PathBuf>,
    /// Source folders removed after a move or delete, parents first.
    pub dirs_to_delete: Vec<PathBuf>,
    /// Entries that cannot be transferred, with the reason.
    pub refused: Vec<FailedFile>,
    /// Entries left out on purpose, with the reason.
    pub skipped: Vec<SkippedFile>,
    pub total_bytes: u64,
}

/// Each of `sources` with the path it goes to: inside `destination` when
/// that is a folder (or there are several sources), `destination` itself
/// otherwise; an empty path for a delete.
pub fn top_targets(
    sources: &[PathBuf],
    destination: &Path,
    operation: TransferOperation,
    kind: DestinationKind,
    dst_fs: &dyn Vfs,
) -> Vec<(PathBuf, PathBuf)> {
    let into_dir = operation != TransferOperation::Delete
        && is_destination_parent_dir(sources, destination, kind, |p| {
            dst_fs.metadata(p).is_ok_and(|e| e.is_dir)
        });
    sources
        .iter()
        .map(|src| {
            let dst = match (operation, into_dir) {
                (TransferOperation::Delete, _) => PathBuf::new(),
                (_, true) => destination.join(src.file_name().unwrap_or_default()),
                (_, false) => destination.to_path_buf(),
            };
            (src.clone(), dst)
        })
        .collect()
}

/// `rel` only goes down (no `..`, root or drive): joined to a destination
/// it stays below it, whatever names a server listed.
fn is_plain_path(rel: &Path) -> bool {
    rel.components().all(|c| matches!(c, Component::Normal(_)))
}

impl ScanOutcome {
    /// Announces the job's totals: what the scan found, plus the `done`
    /// files the job already finished without scanning them (fast
    /// renames), so the progress never reads "3/0".
    pub fn announce(&self, ctl: &JobControl, done: usize) {
        ctl.scan_complete(self.mappings.len() + done, self.total_bytes);
    }
}

/// Scans the sources of `req`, reporting progress through `ctl` (the
/// caller announces the totals, [`ScanOutcome::announce`]). Fails only
/// when the job is cancelled.
pub fn scan(req: &ScanRequest, ctl: &JobControl) -> anyhow::Result<ScanOutcome> {
    ctl.emit(super::events::TransferEvent::ScanProgress {
        job_id: ctl.job_id,
        files_found: 0,
    });
    let mut scanner = Scanner {
        req,
        ctl,
        filter: TransferFilter::of(req.options),
        out: ScanOutcome::default(),
    };
    for (src, dst) in req.roots {
        ctl.ensure_running()?;
        scanner.source(src, dst.clone())?;
    }
    Ok(scanner.out)
}

/// Walks the sources of one job, collecting mappings and totals.
struct Scanner<'a> {
    req: &'a ScanRequest<'a>,
    ctl: &'a JobControl,
    filter: TransferFilter,
    out: ScanOutcome,
}

impl Scanner<'_> {
    fn is_delete(&self) -> bool {
        self.req.operation == TransferOperation::Delete
    }

    /// Move and delete remove the scanned folders afterwards.
    fn removes_dirs(&self) -> bool {
        matches!(
            self.req.operation,
            TransferOperation::Delete | TransferOperation::Move
        )
    }

    /// How links are treated: a delete removes links and never follows them
    /// (moves never follow them either, see
    /// [`TransferOptions::for_operation`]).
    fn link_policy(&self) -> LinkPolicy {
        let options = self.req.options;
        if self.is_delete() {
            LinkPolicy::Keep
        } else if options.skip_symlinks {
            LinkPolicy::Skip
        } else if options.follow_symlinks {
            LinkPolicy::Follow
        } else {
            LinkPolicy::Keep
        }
    }

    fn push(&mut self, src: PathBuf, dst: PathBuf, size: u64) {
        self.out.total_bytes += size;
        self.out.mappings.push((src, dst, size));
    }

    fn refuse(&mut self, src: &Path, error: String) {
        self.out.refused.push(failed(src, PathBuf::new(), error));
    }

    fn skip(&mut self, src: &Path, reason: String) {
        self.out.skipped.push(SkippedFile {
            src: src.to_path_buf(),
            reason,
        });
    }

    fn progress(&self) {
        self.ctl.emit(super::events::TransferEvent::ScanProgress {
            job_id: self.ctl.job_id,
            files_found: self.out.mappings.len(),
        });
    }

    /// One source of the job, transferred to `dst`.
    fn source(&mut self, src: &Path, dst: PathBuf) -> anyhow::Result<()> {
        let entry = match self.req.src_fs.stat(src) {
            Ok(entry) => entry,
            Err(e) => {
                self.out.refused.push(failed_io(src, PathBuf::new(), &e));
                return Ok(());
            }
        };
        let policy = self.link_policy();
        if entry.is_symlink && policy == LinkPolicy::Skip {
            return Ok(());
        }
        let enters = if entry.is_symlink {
            policy == LinkPolicy::Follow && self.req.src_fs.metadata(src).is_ok_and(|e| e.is_dir)
        } else {
            entry.is_dir
        };
        if !enters {
            self.single(entry, dst);
        } else if self.copies_into_itself(src, &dst) {
            self.refuse(
                src,
                t("error_copy_into_itself").replacen("{}", &src.to_string_lossy(), 1),
            );
        } else {
            self.tree(src, &dst)?;
        }
        self.progress();
        Ok(())
    }

    /// `true` when the destination folder lies inside (or is) the source
    /// folder: copying would walk its own copies forever.
    fn copies_into_itself(&self, src: &Path, dst: &Path) -> bool {
        !self.is_delete() && self.req.same_fs && guard::is_within(self.req.dst_fs, dst, src)
    }

    /// A source file, or a link that is not entered.
    fn single(&mut self, entry: VfsEntry, dst: PathBuf) {
        if entry.is_symlink && self.link_policy() == LinkPolicy::Keep {
            return self.kept_link(entry, dst);
        }
        let (size, modified) = self
            .req
            .src_fs
            .metadata(&entry.path)
            .map_or((0, None), |e| (e.size, e.modified));
        if self.filter.matches(&entry.path, size, modified) {
            self.push(entry.path, dst, size);
        }
    }

    /// A link transferred without following it.
    fn kept_link(&mut self, link: VfsEntry, dst: PathBuf) {
        if self.is_delete() || self.req.dst_links {
            return self.push(link.path, dst, 0);
        }
        // Links cannot be made there: copy what a file link points to.
        match self.req.src_fs.metadata(&link.path) {
            Ok(target) if !target.is_dir => self.push(link.path, dst, target.size),
            Ok(_) => {
                let reason = t("transfer_link_dir_skipped").replacen("{}", &link.name, 1);
                self.skip(&link.path, reason);
            }
            Err(_) => {
                let reason = t("transfer_link_broken_skipped").replacen("{}", &link.name, 1);
                self.skip(&link.path, reason);
            }
        }
    }

    /// A source folder and everything below it.
    fn tree(&mut self, src: &Path, base_dst: &Path) -> anyhow::Result<()> {
        if self.removes_dirs() {
            self.out.dirs_to_delete.push(src.to_path_buf());
        }
        if !self.is_delete() {
            self.out.dst_dirs.push(base_dst.to_path_buf());
        }
        let ctl = self.ctl;
        let cancelled = || ctl.is_cancelled();
        let options = WalkOptions::with_links(self.link_policy());
        let mut walker = Walker::new(self.req.src_fs, src, options).with_cancel(&cancelled);
        while let Some(item) = walker.next() {
            match item {
                Err(WalkError::Cancelled) => return Err(crate::fs::error::cancelled()),
                Err(e) => self.out.refused.push(walk_failed(&e)),
                Ok(item) if item.descends && self.filter.excludes(&item.entry.name) => {
                    walker.skip_dir();
                }
                Ok(item) => self.found(item, src, base_dst),
            }
        }
        Ok(())
    }

    /// Records one walked entry of the folder `src`.
    fn found(&mut self, item: WalkEntry, src: &Path, base_dst: &Path) {
        let dst = match (self.is_delete(), item.path().strip_prefix(src)) {
            (true, _) => PathBuf::new(),
            (false, Ok(rel)) if is_plain_path(rel) => base_dst.join(rel),
            (false, Ok(_)) => {
                let name = item.path().to_string_lossy().into_owned();
                return self.refuse(
                    item.path(),
                    t("transfer_unsafe_name").replacen("{}", &name, 1),
                );
            }
            (false, Err(_)) => return,
        };
        if item.descends {
            if self.removes_dirs() {
                self.out.dirs_to_delete.push(item.entry.path.clone());
            }
            if !self.is_delete() {
                self.out.dst_dirs.push(dst);
            }
            return self.progress();
        }
        if item.is_kept_link() {
            return self.kept_link(item.entry, dst);
        }
        let entry = &item.entry;
        if self.filter.matches(&entry.path, entry.size, entry.modified) {
            self.push(item.entry.path, dst, item.entry.size);
        }
    }
}
