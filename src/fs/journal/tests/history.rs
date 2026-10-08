use super::super::history::JOURNAL_CAPACITY;
use super::super::{FsCommand, JobOrigin, Journal};
use super::UNDO;
use crate::fs::journal::Direction;
use std::path::PathBuf;

fn mkdir(name: &str) -> FsCommand {
    FsCommand::MakeDir {
        path: PathBuf::from(name),
    }
}

fn path_of(command: Option<&FsCommand>) -> Option<PathBuf> {
    match command? {
        FsCommand::MakeDir { path } | FsCommand::RemoveDir { path } => Some(path.clone()),
        _ => None,
    }
}

#[test]
fn journal_is_bounded_and_newest_first() {
    let mut journal = Journal::default();
    for i in 0..JOURNAL_CAPACITY + 5 {
        journal.record(mkdir(&i.to_string()));
    }
    assert_eq!(journal.len(UNDO), JOURNAL_CAPACITY);
    let newest = (JOURNAL_CAPACITY + 4).to_string();
    assert_eq!(path_of(journal.peek(UNDO)), Some(PathBuf::from(newest)));
}

#[test]
fn applied_undo_goes_to_redo_and_a_new_operation_clears_it() {
    let mut journal = Journal::default();
    journal.record(mkdir("a"));
    let entry = journal.take(UNDO).unwrap();
    journal.settle(Some(UNDO), entry.inverse().unwrap());
    assert_eq!(journal.len(UNDO), 0);
    assert_eq!(path_of(journal.peek(Direction::Redo)), Some("a".into()));

    let redo = journal.take(Direction::Redo).unwrap();
    journal.settle(Some(Direction::Redo), redo.inverse().unwrap());
    assert!(matches!(
        journal.peek(UNDO),
        Some(FsCommand::MakeDir { .. })
    ));

    journal.take(UNDO);
    journal.applied(UNDO, mkdir("a").inverse().unwrap());
    journal.record(mkdir("b"));
    assert_eq!(journal.len(Direction::Redo), 0, "new operation clears redo");
}

#[test]
fn empty_commands_are_not_journaled() {
    let mut journal = Journal::default();
    journal.record(FsCommand::Trash { paths: Vec::new() });
    journal.applied(UNDO, FsCommand::Restore { paths: Vec::new() });
    assert_eq!(journal.len(UNDO), 0);
    assert_eq!(journal.len(Direction::Redo), 0);
}

#[test]
fn jobs_are_attributed_once() {
    let mut journal = Journal::default();
    let undo_job = uuid::Uuid::new_v4();
    journal.begin_job(undo_job, UNDO, mkdir("x"));
    assert!(matches!(
        journal.finish_job(undo_job),
        JobOrigin::Journal(Direction::Undo, _)
    ));
    assert!(matches!(journal.finish_job(undo_job), JobOrigin::Seen));
    assert!(matches!(
        journal.finish_job(uuid::Uuid::new_v4()),
        JobOrigin::User
    ));

    journal.begin_rename(Direction::Redo);
    assert_eq!(journal.finish_rename(), Some(Direction::Redo));
    assert_eq!(journal.finish_rename(), None);
}

#[test]
fn irreversible_entries_have_no_inverse() {
    let entry = FsCommand::NotUndoable {
        kind: super::super::command::Irreversible::Delete,
        count: 2,
    };
    assert!(entry.inverse().is_none());
    assert!(!entry.is_undoable());
    assert!(entry.label().contains('2'));
}

#[test]
fn long_names_are_shortened_in_labels() {
    let label = mkdir(&"x".repeat(80)).label();
    assert!(label.chars().count() < 50, "{label}");
    assert!(label.ends_with("…»"));
}
