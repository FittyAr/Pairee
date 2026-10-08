//! Ordering, execution and rollback tests.

use super::tests::{CS, WIN, counter_rules, names};
use super::*;
use crate::fs::vfs::{Capabilities, LocalVfs, Vfs, VfsEntry};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn p(name: &str) -> PathBuf {
    PathBuf::from(name)
}

fn moves(pairs: &[(&str, &str)]) -> Vec<(PathBuf, PathBuf)> {
    pairs.iter().map(|(a, b)| (p(a), p(b))).collect()
}

fn nothing_taken(_: &Path) -> bool {
    false
}

fn pairs(steps: &[Step]) -> Vec<(PathBuf, PathBuf)> {
    steps
        .iter()
        .map(|s| (s.from.clone(), s.to.clone()))
        .collect()
}

#[test]
fn chains_run_from_the_free_end() {
    let steps = plan(
        &moves(&[("1", "2"), ("2", "3"), ("3", "4")]),
        CS,
        &nothing_taken,
    )
    .unwrap();
    assert_eq!(pairs(&steps), moves(&[("3", "4"), ("2", "3"), ("1", "2")]));
}

#[test]
fn swaps_go_through_a_temporary_name() {
    let steps = plan(&moves(&[("a", "b"), ("b", "a")]), CS, &nothing_taken).unwrap();
    assert_eq!(steps.len(), 3);
    let temp = steps[0].to.clone();
    assert_eq!(steps[0].from, p("a"));
    assert!(crate::fs::file_name_lossy(&temp).contains("pairee-tmp"));
    assert_eq!(pairs(&steps[1..]), vec![(p("b"), p("a")), (temp, p("b"))]);
}

#[test]
fn longer_cycles_and_independent_moves() {
    let cycle = moves(&[("a", "b"), ("b", "c"), ("c", "a"), ("x", "y")]);
    let steps = plan(&cycle, CS, &nothing_taken).unwrap();
    assert_eq!(steps.len(), 5, "4 moves + 1 temporary");
    assert_eq!(
        simulate(&["a", "b", "c", "x"], &steps),
        ["a<-c", "b<-a", "c<-b", "y<-x"]
    );
}

#[test]
fn temporary_names_avoid_existing_entries() {
    let taken = |path: &Path| crate::fs::file_name_lossy(path) == "a.pairee-tmp1";
    let steps = plan(&moves(&[("a", "b"), ("b", "a")]), CS, &taken).unwrap();
    assert_eq!(steps[0].to, p("a.pairee-tmp2"));
}

#[test]
fn case_only_renames_and_duplicate_targets() {
    let steps = plan(&moves(&[("a", "A"), ("same", "same")]), WIN, &nothing_taken).unwrap();
    assert_eq!(
        pairs(&steps),
        moves(&[("a", "A")]),
        "unchanged entries skipped"
    );
    assert!(plan(&moves(&[("a", "x"), ("b", "X")]), WIN, &nothing_taken).is_err());
    assert!(plan(&moves(&[("a", "x"), ("b", "X")]), CS, &nothing_taken).is_ok());
}

/// In-memory backend (current name, original name). Renaming onto an
/// existing name overwrites it, like POSIX `rename`, so ordering bugs show.
#[derive(Debug)]
struct MemFs {
    entries: Mutex<Vec<(String, String)>>,
    /// Targets whose rename fails, each entry consumed once.
    failures: Mutex<Vec<&'static str>>,
    log: Mutex<Vec<String>>,
}

impl MemFs {
    fn new(names: &[&str]) -> Self {
        Self {
            entries: Mutex::new(
                names
                    .iter()
                    .map(|n| (n.to_string(), n.to_string()))
                    .collect(),
            ),
            failures: Mutex::default(),
            log: Mutex::default(),
        }
    }

    fn names(&self) -> HashSet<String> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .map(|(n, _)| n.clone())
            .collect()
    }
}

/// Only `rename` and `exists` are used by [`execute`].
fn not_listed() -> io::Error {
    io::Error::from(io::ErrorKind::Unsupported)
}

impl Vfs for MemFs {
    fn capabilities(&self) -> Capabilities {
        Capabilities::READ_ONLY
    }

    fn list(&self, _dir: &Path) -> io::Result<Vec<VfsEntry>> {
        Err(not_listed())
    }

    fn stat(&self, _path: &Path) -> io::Result<VfsEntry> {
        Err(not_listed())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let from = crate::fs::file_name_lossy(from);
        let to = crate::fs::file_name_lossy(to);
        self.log.lock().unwrap().push(format!("{from}->{to}"));
        let mut failures = self.failures.lock().unwrap();
        if let Some(idx) = failures.iter().position(|f| *f == to) {
            failures.remove(idx);
            return Err(io::Error::other("boom"));
        }
        let mut entries = self.entries.lock().unwrap();
        let idx = entries
            .iter()
            .position(|(n, _)| *n == from)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let original = entries.remove(idx).1;
        entries.retain(|(n, _)| *n != to);
        entries.push((to, original));
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.names().contains(&crate::fs::file_name_lossy(path))
    }
}

/// Applies `steps`; returns sorted `"now<-originally"` labels.
fn simulate(initial: &[&str], steps: &[Step]) -> Vec<String> {
    let backend = MemFs::new(initial);
    let report = execute(steps, &backend, CS);
    assert!(report.is_success(), "{report:?}");
    let mut out: Vec<String> = backend
        .entries
        .lock()
        .unwrap()
        .iter()
        .map(|(now, was)| format!("{now}<-{was}"))
        .collect();
    out.sort();
    out
}

#[test]
fn swap_executes_without_losing_files() {
    let steps = plan(&moves(&[("a", "b"), ("b", "a")]), CS, &nothing_taken).unwrap();
    assert_eq!(simulate(&["a", "b"], &steps), ["a<-b", "b<-a"]);
}

#[test]
fn failure_rolls_back_every_applied_step() {
    let steps = plan(&moves(&[("1", "2"), ("2", "3")]), CS, &nothing_taken).unwrap();
    let backend = MemFs::new(&["1", "2"]);
    backend.failures.lock().unwrap().push("2"); // the second step (1 -> 2) fails
    let report = execute(&steps, &backend, CS);
    assert!(!report.is_success());
    assert_eq!(
        report.failure.as_ref().map(|f| f.path.clone()),
        Some(p("1"))
    );
    assert!(report.not_rolled_back.is_empty());
    assert!(report.applied.is_empty());
    assert_eq!(backend.names(), HashSet::from(["1".into(), "2".into()]));
    assert_eq!(*backend.log.lock().unwrap(), ["2->3", "1->2", "3->2"]);
}

#[test]
fn rollback_failures_are_reported() {
    let steps = plan(&moves(&[("1", "2"), ("2", "3")]), CS, &nothing_taken).unwrap();
    let backend = MemFs::new(&["1", "2"]);
    // 1 -> 2 fails, and so does undoing 2 -> 3 (3 -> 2).
    backend.failures.lock().unwrap().extend(["2", "2"]);
    let report = execute(&steps, &backend, CS);
    assert_eq!(pairs(&report.not_rolled_back), moves(&[("2", "3")]));
    assert_eq!(pairs(&report.applied), moves(&[("2", "3")]));
}

#[test]
fn existing_target_stops_before_overwriting() {
    let steps = [Step {
        from: p("a"),
        to: p("b"),
    }];
    let backend = MemFs::new(&["a", "b"]);
    let report = execute(&steps, &backend, CS);
    assert!(!report.is_success());
    assert!(
        backend.log.lock().unwrap().is_empty(),
        "nothing was renamed"
    );
}

#[test]
fn case_only_rename_ignores_its_own_entry() {
    /// Case-insensitive store: `exists` matches any case.
    #[derive(Debug)]
    struct Ci(MemFs);
    impl Vfs for Ci {
        fn capabilities(&self) -> Capabilities {
            Capabilities::READ_ONLY
        }
        fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
            self.0.list(dir)
        }
        fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
            self.0.stat(path)
        }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            self.0.rename(from, to)
        }
        fn exists(&self, path: &Path) -> bool {
            let name = crate::fs::file_name_lossy(path).to_lowercase();
            self.0.names().iter().any(|n| n.to_lowercase() == name)
        }
    }
    let backend = Ci(MemFs::new(&["a"]));
    let steps = [Step {
        from: p("a"),
        to: p("A"),
    }];
    assert!(execute(&steps, &backend, WIN).is_success());
    assert_eq!(backend.0.names(), HashSet::from(["A".into()]));
}

#[test]
fn local_backend_swaps_real_files() {
    let dir = tempfile::tempdir().unwrap();
    let sources: Vec<RenameSource> = ["1.txt", "2.txt"]
        .iter()
        .map(|name| {
            let path = dir.path().join(name);
            std::fs::write(&path, *name).unwrap();
            RenameSource {
                path,
                is_dir: false,
                modified: None,
            }
        })
        .collect();
    let fs = TargetFs::local();
    let rules = counter_rules(2, -1).compile().unwrap();
    let preview = Preview::build(&sources, &rules, &names(&["1.txt", "2.txt"]), fs);
    let steps = plan(&preview.moves(&sources), fs, &|path: &Path| {
        LocalVfs.exists(path)
    })
    .unwrap();
    let report = execute(&steps, &LocalVfs, fs);
    assert!(report.is_success(), "{report:?}");
    let read = |name: &str| std::fs::read_to_string(dir.path().join(name)).unwrap();
    assert_eq!(read("1.txt"), "2.txt");
    assert_eq!(read("2.txt"), "1.txt");
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        2,
        "no temp left"
    );
}
