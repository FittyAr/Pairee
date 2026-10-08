//! Data model of a directory synchronization: direction, per-item
//! difference and the action chosen for it.

use crate::fs::compare::{CompareEntry, CompareStatus, FileSummary};
use std::path::PathBuf;

/// Which way differences are resolved by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncDirection {
    /// Copy what is new or changed on the left to the right.
    #[default]
    LeftToRight,
    /// Copy what is new or changed on the right to the left.
    RightToLeft,
    /// Copy in both directions; the newer file wins.
    Both,
    /// Make the right side identical to the left (deletes extras on the right).
    Mirror,
}

impl SyncDirection {
    pub const ALL: [Self; 4] = [
        Self::LeftToRight,
        Self::RightToLeft,
        Self::Both,
        Self::Mirror,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::LeftToRight => "sync_direction_left_to_right",
            Self::RightToLeft => "sync_direction_right_to_left",
            Self::Both => "sync_direction_both",
            Self::Mirror => "sync_direction_mirror",
        }
    }

    /// The next direction (cycling).
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|d| *d == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// How the two sides of one path differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Equal,
    OnlyLeft,
    OnlyRight,
    /// Both exist; the left file is newer.
    LeftNewer,
    /// Both exist; the right file is newer.
    RightNewer,
    /// Both exist with the same time but different size or content (or,
    /// for a folder pair, something inside differs).
    Differs,
    /// A file on one side, a folder on the other.
    TypeMismatch,
}

impl DiffKind {
    /// Coarse status used by the folder-compare result list.
    pub fn compare_status(self) -> CompareStatus {
        match self {
            Self::Equal => CompareStatus::Equal,
            Self::OnlyLeft => CompareStatus::OnlyLeft,
            Self::OnlyRight => CompareStatus::OnlyRight,
            Self::LeftNewer | Self::RightNewer | Self::Differs | Self::TypeMismatch => {
                CompareStatus::Different
            }
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Equal => "sync_kind_equal",
            Self::OnlyLeft => "sync_kind_only_left",
            Self::OnlyRight => "sync_kind_only_right",
            Self::LeftNewer => "sync_kind_left_newer",
            Self::RightNewer => "sync_kind_right_newer",
            Self::Differs => "sync_kind_differs",
            Self::TypeMismatch => "sync_kind_type_mismatch",
        }
    }
}

/// What applying the plan does with one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    Skip,
    CopyToRight,
    CopyToLeft,
    DeleteLeft,
    DeleteRight,
}

impl SyncAction {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Skip => "sync_action_skip",
            Self::CopyToRight => "sync_action_copy_right",
            Self::CopyToLeft => "sync_action_copy_left",
            Self::DeleteLeft => "sync_action_delete_left",
            Self::DeleteRight => "sync_action_delete_right",
        }
    }

    pub fn is_delete(self) -> bool {
        matches!(self, Self::DeleteLeft | Self::DeleteRight)
    }
}

/// One compared path and the action planned for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItem {
    /// Path relative to the compared roots (left-side spelling).
    pub rel_path: PathBuf,
    /// Absolute path on the left (where it is, or would be created).
    pub left_path: PathBuf,
    /// Absolute path on the right (where it is, or would be created).
    pub right_path: PathBuf,
    pub left: Option<FileSummary>,
    pub right: Option<FileSummary>,
    pub kind: DiffKind,
    pub action: SyncAction,
    /// Bytes on the left / right (a folder on one side only: its files'
    /// total).
    pub left_bytes: u64,
    pub right_bytes: u64,
}

impl SyncItem {
    /// A folder on both sides: only shown as context, never acted on.
    pub fn is_dir_pair(&self) -> bool {
        matches!((&self.left, &self.right), (Some(l), Some(r)) if l.is_dir && r.is_dir)
    }

    /// True when the item is a folder (on the side or sides it exists).
    pub fn is_dir(&self) -> bool {
        self.left.or(self.right).is_some_and(|s| s.is_dir)
    }

    /// Bytes moved or freed by the current action.
    pub fn action_bytes(&self) -> u64 {
        match self.action {
            SyncAction::CopyToRight | SyncAction::DeleteLeft => self.left_bytes,
            SyncAction::CopyToLeft | SyncAction::DeleteRight => self.right_bytes,
            SyncAction::Skip => 0,
        }
    }
}

/// Folder-compare rows: the top-level items, folder pairs rolled up (a
/// folder present on both sides is "different" when anything inside is).
pub fn compare_entries(items: &[SyncItem]) -> Vec<CompareEntry> {
    items
        .iter()
        .filter(|item| item.rel_path.components().count() == 1)
        .map(|item| CompareEntry {
            name: item.rel_path.to_string_lossy().into_owned(),
            status: item.kind.compare_status(),
        })
        .collect()
}
