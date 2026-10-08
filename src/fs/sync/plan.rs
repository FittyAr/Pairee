//! Turning differences into actions, and actions into Transfer Engine jobs.

use super::model::{DiffKind, SyncAction, SyncDirection, SyncItem};
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Conflict policy of sync copies (see `TransferOptions::conflict_resolution`).
const OVERWRITE: &str = "overwrite";

/// The action a direction picks for a difference.
pub fn default_action(kind: DiffKind, direction: SyncDirection) -> SyncAction {
    use DiffKind as K;
    use SyncAction as A;
    use SyncDirection as D;
    match (direction, kind) {
        (_, K::Equal | K::TypeMismatch) => A::Skip,
        (D::LeftToRight | D::Mirror | D::Both, K::OnlyLeft) => A::CopyToRight,
        (D::RightToLeft | D::Both, K::OnlyRight) => A::CopyToLeft,
        (D::Mirror, K::OnlyRight) => A::DeleteRight,
        (D::LeftToRight | D::Mirror | D::Both, K::LeftNewer) => A::CopyToRight,
        (D::RightToLeft | D::Both, K::RightNewer) => A::CopyToLeft,
        // A one-way copy never overwrites a newer target unless mirroring.
        (D::Mirror, K::RightNewer | K::Differs) => A::CopyToRight,
        (D::LeftToRight, K::Differs) => A::CopyToRight,
        (D::RightToLeft, K::Differs) => A::CopyToLeft,
        _ => A::Skip,
    }
}

/// Actions that make sense for an item (the first is never `Skip` unless
/// it is the only one).
pub fn allowed_actions(item: &SyncItem) -> &'static [SyncAction] {
    use SyncAction as A;
    if item.is_dir_pair() {
        return &[A::Skip];
    }
    match item.kind {
        DiffKind::Equal | DiffKind::TypeMismatch => &[A::Skip],
        DiffKind::OnlyLeft => &[A::CopyToRight, A::DeleteLeft, A::Skip],
        DiffKind::OnlyRight => &[A::CopyToLeft, A::DeleteRight, A::Skip],
        DiffKind::LeftNewer | DiffKind::RightNewer | DiffKind::Differs => {
            &[A::CopyToRight, A::CopyToLeft, A::Skip]
        }
    }
}

/// Sets `action` on `item` when it is allowed; returns whether it was.
pub fn set_action(item: &mut SyncItem, action: SyncAction) -> bool {
    let allowed = allowed_actions(item).contains(&action);
    if allowed {
        item.action = action;
    }
    allowed
}

/// The delete action for an item that exists on one side only.
pub fn delete_action(item: &SyncItem) -> Option<SyncAction> {
    match item.kind {
        DiffKind::OnlyLeft => Some(SyncAction::DeleteLeft),
        DiffKind::OnlyRight => Some(SyncAction::DeleteRight),
        _ => None,
    }
}

/// Moves `item` to its next allowed action.
pub fn cycle_action(item: &mut SyncItem) {
    let allowed = allowed_actions(item);
    let i = allowed.iter().position(|a| *a == item.action).unwrap_or(0);
    item.action = allowed[(i + 1) % allowed.len()];
}

/// Re-applies the direction's default action to every item.
pub fn apply_direction(items: &mut [SyncItem], direction: SyncDirection) {
    for item in items {
        item.action = if item.is_dir_pair() {
            SyncAction::Skip
        } else {
            default_action(item.kind, direction)
        };
    }
}

/// Totals of the planned actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncSummary {
    pub copy_right: usize,
    pub copy_right_bytes: u64,
    pub copy_left: usize,
    pub copy_left_bytes: u64,
    pub delete: usize,
    pub delete_bytes: u64,
    pub skipped: usize,
    pub equal: usize,
}

impl SyncSummary {
    pub fn of(items: &[SyncItem]) -> Self {
        let mut s = Self::default();
        for item in items.iter().filter(|i| !i.is_dir_pair()) {
            let bytes = item.action_bytes();
            match item.action {
                SyncAction::CopyToRight => {
                    s.copy_right += 1;
                    s.copy_right_bytes += bytes;
                }
                SyncAction::CopyToLeft => {
                    s.copy_left += 1;
                    s.copy_left_bytes += bytes;
                }
                SyncAction::DeleteLeft | SyncAction::DeleteRight => {
                    s.delete += 1;
                    s.delete_bytes += bytes;
                }
                SyncAction::Skip if item.kind == DiffKind::Equal => s.equal += 1,
                SyncAction::Skip => s.skipped += 1,
            }
        }
        s
    }

    pub fn has_work(&self) -> bool {
        self.copy_right + self.copy_left + self.delete > 0
    }
}

/// One Transfer Engine job of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedJob {
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    /// Copy: target folder (several sources) or target path (one source).
    /// Delete: empty.
    pub destination: PathBuf,
}

impl PlannedJob {
    /// The engine job: copies overwrite (the user chose them), keep the
    /// source times (so the next comparison sees equal files) and honour
    /// `mask` inside folders that exist on one side only.
    pub fn into_transfer_job(self, base: &TransferOptions, mask: &str) -> TransferJob {
        let options = TransferOptions {
            conflict_resolution: OVERWRITE.to_owned(),
            preserve_timestamps: true,
            filter_mask: (!mask.is_empty()).then(|| mask.to_owned()),
            ..base.clone()
        };
        TransferJob::new(self.operation, self.sources, self.destination, options)
    }
}

/// Groups the actions into Transfer Engine jobs: copies sharing source and
/// target folders become one job, deletes become one job per side.
pub fn plan_jobs(items: &[SyncItem]) -> Vec<PlannedJob> {
    let mut copies: BTreeMap<(PathBuf, PathBuf), Vec<(PathBuf, PathBuf)>> = BTreeMap::new();
    let mut deletes: [Vec<PathBuf>; 2] = Default::default();
    for item in items {
        let (src, dst) = match item.action {
            SyncAction::CopyToRight => (&item.left_path, &item.right_path),
            SyncAction::CopyToLeft => (&item.right_path, &item.left_path),
            SyncAction::DeleteLeft => {
                deletes[0].push(item.left_path.clone());
                continue;
            }
            SyncAction::DeleteRight => {
                deletes[1].push(item.right_path.clone());
                continue;
            }
            SyncAction::Skip => continue,
        };
        copies
            .entry((parent(src), parent(dst)))
            .or_default()
            .push((src.clone(), dst.clone()));
    }
    let mut jobs: Vec<PlannedJob> = copies
        .into_iter()
        .map(|((_, dst_dir), pairs)| copy_job(dst_dir, pairs))
        .collect();
    jobs.extend(
        deletes
            .into_iter()
            .filter(|sources| !sources.is_empty())
            .map(|sources| PlannedJob {
                operation: TransferOperation::Delete,
                sources,
                destination: PathBuf::new(),
            }),
    );
    jobs
}

/// One source is copied onto its exact target path; several go into their
/// common target folder.
fn copy_job(dst_dir: PathBuf, mut pairs: Vec<(PathBuf, PathBuf)>) -> PlannedJob {
    let destination = match pairs.as_slice() {
        [(_, dst)] => dst.clone(),
        _ => dst_dir,
    };
    pairs.sort();
    PlannedJob {
        operation: TransferOperation::Copy,
        sources: pairs.into_iter().map(|(src, _)| src).collect(),
        destination,
    }
}

fn parent(path: &Path) -> PathBuf {
    path.parent().map(Path::to_path_buf).unwrap_or_default()
}
