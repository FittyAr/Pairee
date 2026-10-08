//! Pure helpers for panel listings: mask filtering, quick-filter partition,
//! auto-update throttling and git pathspec scoping.

use super::listing::{child_suffix, normalize};
use crate::fs::FileEntry;
use std::path::Path;

/// Applies the permanent glob mask and the quick-filter partition.
pub fn apply_masks(
    mut entries: Vec<FileEntry>,
    filter_mask: Option<&str>,
    quick_filter_mask: Option<&str>,
) -> Vec<FileEntry> {
    if let Some(mask) = filter_mask.filter(|m| !m.is_empty()) {
        entries.retain(|e| e.name == ".." || crate::app::state::glob::glob_matches(mask, &e.name));
    }
    match quick_filter_mask.filter(|m| !m.is_empty()) {
        Some(qmask) => partition_entries_by_mask(entries, qmask),
        None => entries,
    }
}

/// Keeps `..` first, then entries whose name contains `mask`, then the rest.
pub fn partition_entries_by_mask(entries: Vec<FileEntry>, mask: &str) -> Vec<FileEntry> {
    let mask_lower = mask.to_lowercase();
    let mut dotdot = Vec::new();
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();
    for entry in entries {
        if entry.name == ".." {
            dotdot.push(entry);
        } else if entry.name.to_lowercase().contains(&mask_lower) {
            matching.push(entry);
        } else {
            non_matching.push(entry);
        }
    }
    dotdot.truncate(1);
    dotdot.extend(matching);
    dotdot.extend(non_matching);
    dotdot
}

/// True when an automatic reread of the *same* directory should be skipped
/// because it already holds more than `limit` objects (0 disables the limit).
pub fn skip_auto_update(limit: u32, current_count: usize, must_load: bool) -> bool {
    !must_load && limit > 0 && current_count > limit as usize
}

/// Repository-relative pathspec (forward slashes) restricting `git status`
/// to `dir`. `None` means "whole repository": `dir` is the work tree root,
/// lies outside it, or contains glob metacharacters a pathspec would expand.
pub fn git_pathspec_for(workdir: &Path, dir: &Path) -> Option<String> {
    let dir_slashed = dir.to_string_lossy().replace('\\', "/");
    let dir_norm = normalize(dir);
    // Lower-casing must not shift byte offsets, or slicing below is invalid.
    if dir_norm.len() != dir_slashed.len() {
        return None;
    }
    let suffix = child_suffix(&dir_norm, &normalize(workdir))?;
    let start = dir_norm.len() - suffix.len();
    let rel = dir_slashed.get(start..)?.trim_end_matches('/');
    if rel.is_empty() || rel.contains(['*', '?', '[', ']']) {
        return None;
    }
    Some(rel.to_string())
}
