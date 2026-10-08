use super::super::check;
use super::{FsCommand, p, read, runnable_inverse, write};
use crate::fs::multi_rename::{LocalFs, Step, TargetFs, execute, plan};
use std::path::{Path, PathBuf};

/// Runs `moves` like the multi-rename tool and returns the journal entry.
fn rename_batch(moves: &[(PathBuf, PathBuf)]) -> FsCommand {
    let fs = TargetFs::local();
    let steps = plan(moves, fs, &|path: &Path| path.exists()).unwrap();
    let report = execute(&steps, &LocalFs, fs);
    assert!(report.is_success(), "{report:?}");
    FsCommand::Rename {
        steps: report.applied,
        ssh: None,
    }
}

/// Runs a rename command as the undo executor does.
fn run(command: &FsCommand) -> FsCommand {
    let FsCommand::Rename { steps, .. } = command else {
        panic!("not a rename: {command:?}");
    };
    let report = execute(steps, &LocalFs, TargetFs::local());
    assert!(report.is_success(), "{report:?}");
    FsCommand::Rename {
        steps: report.applied,
        ssh: None,
    }
}

#[test]
fn swap_undo_and_redo_restore_both_names() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (p(dir.path(), "a.txt"), p(dir.path(), "b.txt"));
    write(&a, "A");
    write(&b, "B");
    let entry = rename_batch(&[(a.clone(), b.clone()), (b.clone(), a.clone())]);
    assert_eq!(read(&a), "B");

    let undone = run(&runnable_inverse(&entry));
    assert_eq!((read(&a), read(&b)), ("A".into(), "B".into()));

    run(&runnable_inverse(&undone));
    assert_eq!((read(&a), read(&b)), ("B".into(), "A".into()));
}

#[test]
fn rename_chain_is_undone_in_reverse_order() {
    let dir = tempfile::tempdir().unwrap();
    let names = ["1", "2", "3"].map(|n| p(dir.path(), n));
    write(&names[0], "one");
    write(&names[1], "two");
    // 2 → 3 must run before 1 → 2.
    let entry = rename_batch(&[
        (names[0].clone(), names[1].clone()),
        (names[1].clone(), names[2].clone()),
    ]);
    assert_eq!(read(&names[2]), "two");
    run(&runnable_inverse(&entry));
    assert_eq!(read(&names[0]), "one");
    assert_eq!(read(&names[1]), "two");
    assert!(!names[2].exists());
}

#[test]
fn rename_undo_is_refused_when_the_old_name_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = (p(dir.path(), "old"), p(dir.path(), "new"));
    write(&new, "renamed");
    let entry = FsCommand::Rename {
        steps: vec![Step {
            from: old.clone(),
            to: new.clone(),
        }],
        ssh: None,
    };
    write(&old, "someone else");
    let checked = check(&entry.inverse().unwrap());
    assert!(checked.runnable.is_none());
    assert_eq!(checked.skipped[0].path, old);
    assert_eq!(read(&old), "someone else", "nothing was touched");
}

#[test]
fn rename_undo_is_refused_when_the_file_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    let entry = FsCommand::Rename {
        steps: vec![Step {
            from: p(dir.path(), "a"),
            to: p(dir.path(), "b"),
        }],
        ssh: None,
    };
    let checked = check(&entry.inverse().unwrap());
    assert!(checked.runnable.is_none());
    assert_eq!(checked.skipped[0].path, p(dir.path(), "b"));
}
