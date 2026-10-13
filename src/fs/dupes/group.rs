//! Hash passes of the duplicate finder: each splits the candidate groups
//! by a hash of (part of) their files and keeps the groups that still have
//! two files or more.

use super::scan::REPORT_EVERY;
use super::{DupeControl, DupeGroup, DupeOptions, DupeProgress, DupeReport, DupeStage};
use crate::fs::transfer::hash::hash_reader;
use crate::fs::vfs::Vfs;
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Files that may have the same contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub size: u64,
    /// Hash the files share so far (`None` before the first hash pass).
    pub hash: Option<String>,
    pub files: Vec<PathBuf>,
}

/// Runs the hash passes.
pub(super) struct Hasher<'a> {
    vfs: &'a dyn Vfs,
    options: &'a DupeOptions,
    ctl: &'a DupeControl<'a>,
    report: &'a mut DupeReport,
    last_report: Instant,
}

impl<'a> Hasher<'a> {
    pub fn new(
        vfs: &'a dyn Vfs,
        options: &'a DupeOptions,
        ctl: &'a DupeControl<'a>,
        report: &'a mut DupeReport,
    ) -> Self {
        Self {
            vfs,
            options,
            ctl,
            report,
            last_report: Instant::now(),
        }
    }

    /// Splits `groups` by the hash of `stage`. Groups whose files are no
    /// larger than the partial pass reads were hashed whole already: the
    /// full pass keeps them as they are.
    pub fn split(&mut self, groups: Vec<Candidate>, stage: DupeStage) -> Vec<Candidate> {
        let whole = |group: &Candidate| {
            stage == DupeStage::FullHash && group.size <= self.options.partial_bytes
        };
        let total = groups
            .iter()
            .filter(|g| !whole(g))
            .map(|g| g.files.len() as u64)
            .sum();
        let mut progress = DupeProgress {
            stage,
            done: 0,
            total,
            current: PathBuf::new(),
        };
        let mut out = Vec::with_capacity(groups.len());
        for group in groups {
            if (self.ctl.cancelled)() {
                self.report.partial = true;
                break;
            }
            if whole(&group) {
                out.push(group);
                continue;
            }
            let Some(split) = self.split_group(group, stage, &mut progress) else {
                break;
            };
            out.extend(split);
        }
        out
    }

    /// The groups `group` splits into; `None` when the search was cancelled.
    fn split_group(
        &mut self,
        group: Candidate,
        stage: DupeStage,
        progress: &mut DupeProgress,
    ) -> Option<Vec<Candidate>> {
        let limit = match stage {
            DupeStage::PartialHash => self.options.partial_bytes,
            _ => u64::MAX,
        };
        let mut buckets: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for file in group.files {
            progress.done += 1;
            self.report_progress(progress, &file);
            match self.hash(&file, limit) {
                Ok(hash) => buckets.entry(hash).or_default().push(file),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    self.report.partial = true;
                    return None;
                }
                Err(_) => {
                    self.report.partial = true;
                    self.report.unreadable.push(file);
                }
            }
        }
        Some(
            buckets
                .into_iter()
                .filter(|(_, files)| files.len() > 1)
                .map(|(hash, files)| Candidate {
                    size: group.size,
                    hash: Some(hash),
                    files,
                })
                .collect(),
        )
    }

    /// Hash of the first `limit` bytes of `file`.
    fn hash(&self, file: &Path, limit: u64) -> io::Result<String> {
        let mut data = self.vfs.open_read(file)?.take(limit);
        hash_reader(&mut data, self.options.algorithm, self.ctl.cancelled)
    }

    fn report_progress(&mut self, progress: &mut DupeProgress, file: &Path) {
        if self.last_report.elapsed() >= REPORT_EVERY {
            self.last_report = Instant::now();
            progress.current = file.to_path_buf();
            (self.ctl.progress)(progress);
        }
    }
}

/// The confirmed groups, files sorted.
pub(super) fn finish(groups: Vec<Candidate>) -> Vec<DupeGroup> {
    groups
        .into_iter()
        .filter_map(|group| {
            let mut files = group.files;
            files.sort();
            Some(DupeGroup {
                size: group.size,
                hash: group.hash?,
                files,
            })
        })
        .collect()
}
