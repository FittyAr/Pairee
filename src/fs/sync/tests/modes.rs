//! Default actions of every direction, and changing them by hand.

use super::*;
use crate::fs::sync::plan::{
    allowed_actions, apply_direction, cycle_action, delete_action, set_action,
};

/// left: only_l, newer_l (newer), older_l (older), same; right: only_r.
fn fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let (l, r) = trees();
    put(l.path(), "only_l.txt", "123", 0);
    put(l.path(), "newer_l.txt", "new", 600);
    put(r.path(), "newer_l.txt", "old", 0);
    put(l.path(), "older_l.txt", "old", 0);
    put(r.path(), "older_l.txt", "new", 600);
    put(l.path(), "same.txt", "s", 0);
    put(r.path(), "same.txt", "s", 0);
    put(l.path(), "conflict.txt", "aaaa", 0);
    put(r.path(), "conflict.txt", "bb", 0);
    put(r.path(), "only_r.txt", "12345", 0);
    (l, r)
}

fn actions(direction: SyncDirection) -> Vec<(String, SyncAction)> {
    let (l, r) = fixture();
    diff(l.path(), r.path(), &opts(direction))
        .into_iter()
        .map(|i| (i.rel_path.to_string_lossy().into_owned(), i.action))
        .collect()
}

fn expect(direction: SyncDirection, expected: [SyncAction; 6]) {
    let names = [
        "conflict.txt",
        "newer_l.txt",
        "older_l.txt",
        "only_l.txt",
        "only_r.txt",
        "same.txt",
    ];
    let expected: Vec<_> = names.iter().map(|n| n.to_string()).zip(expected).collect();
    assert_eq!(actions(direction), expected, "{direction:?}");
}

use SyncAction::{CopyToLeft as L, CopyToRight as R, DeleteRight as DelR, Skip as S};

#[test]
fn left_to_right_copies_new_and_changed_but_not_over_newer() {
    expect(SyncDirection::LeftToRight, [R, R, S, R, S, S]);
}

#[test]
fn right_to_left_is_symmetric() {
    expect(SyncDirection::RightToLeft, [L, S, L, S, L, S]);
}

#[test]
fn both_directions_newer_wins_and_same_time_conflicts_are_skipped() {
    expect(SyncDirection::Both, [S, R, L, R, L, S]);
}

#[test]
fn mirror_overwrites_everything_and_deletes_extras() {
    expect(SyncDirection::Mirror, [R, R, R, R, DelR, S]);
}

#[test]
fn kinds_are_classified() {
    let (l, r) = fixture();
    let items = diff(l.path(), r.path(), &opts(SyncDirection::Both));
    assert_eq!(find(&items, "conflict.txt").kind, DiffKind::Differs);
    assert_eq!(find(&items, "newer_l.txt").kind, DiffKind::LeftNewer);
    assert_eq!(find(&items, "older_l.txt").kind, DiffKind::RightNewer);
    assert_eq!(find(&items, "only_l.txt").kind, DiffKind::OnlyLeft);
    assert_eq!(find(&items, "only_r.txt").kind, DiffKind::OnlyRight);
    assert_eq!(find(&items, "same.txt").kind, DiffKind::Equal);
    let rows = compare_entries(&items);
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0].status, crate::fs::compare::CompareStatus::Different);
}

#[test]
fn actions_can_be_changed_within_what_is_allowed() {
    let (l, r) = fixture();
    let mut items = diff(l.path(), r.path(), &opts(SyncDirection::LeftToRight));
    let idx = |items: &[SyncItem], rel: &str| {
        items
            .iter()
            .position(|i| i.rel_path == Path::new(rel))
            .unwrap()
    };

    let only_l = idx(&items, "only_l.txt");
    assert!(!set_action(&mut items[only_l], SyncAction::CopyToLeft));
    assert_eq!(delete_action(&items[only_l]), Some(SyncAction::DeleteLeft));
    assert!(set_action(&mut items[only_l], SyncAction::DeleteLeft));
    cycle_action(&mut items[only_l]);
    assert_eq!(items[only_l].action, SyncAction::Skip);
    cycle_action(&mut items[only_l]);
    assert_eq!(items[only_l].action, SyncAction::CopyToRight);

    let same = idx(&items, "same.txt");
    assert_eq!(allowed_actions(&items[same]), [SyncAction::Skip]);
    assert_eq!(delete_action(&items[same]), None);

    let older = idx(&items, "older_l.txt");
    assert!(set_action(&mut items[older], SyncAction::CopyToRight));
    apply_direction(&mut items, SyncDirection::LeftToRight);
    assert_eq!(items[older].action, SyncAction::Skip, "defaults restored");
}

#[test]
fn summary_counts_actions_and_bytes() {
    let (l, r) = fixture();
    let items = diff(l.path(), r.path(), &opts(SyncDirection::Mirror));
    let summary = SyncSummary::of(&items);
    assert_eq!(summary.copy_right, 4);
    assert_eq!(summary.copy_right_bytes, 4 + 3 + 3 + 3);
    assert_eq!(summary.delete, 1);
    assert_eq!(summary.delete_bytes, 5);
    assert_eq!(summary.equal, 1);
    assert_eq!(summary.skipped, 0);
    assert!(summary.has_work());
    assert!(!SyncSummary::of(&[]).has_work());
}

#[test]
fn direction_cycles_through_all_modes() {
    let mut d = SyncDirection::default();
    let mut seen = Vec::new();
    for _ in 0..SyncDirection::ALL.len() {
        seen.push(d);
        d = d.next();
    }
    assert_eq!(seen, SyncDirection::ALL);
    assert_eq!(d, SyncDirection::default());
}
