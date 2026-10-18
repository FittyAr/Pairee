//! Archive jobs honour the transfer dialog's filter, announce the number
//! of entries they actually write and report real paths (the report file
//! and the eject post-action go where the job wrote).

use super::run_job;
use crate::fs::archive::list_archive_files;
use crate::fs::archive::test_fixtures::{SAMPLE_TREE, snapshot, write_zip};
use crate::fs::transfer::events::{EventSender, TransferEvent};
use crate::fs::transfer::job::{TransferJob, TransferOperation, TransferResults};
use crate::fs::transfer::options::TransferOptions;
use std::fs;
use std::path::Path;

fn filtered(mask: &str) -> TransferOptions {
    TransferOptions {
        filter_mask: Some(mask.into()),
        ..TransferOptions::default()
    }
}

/// Runs `job` to the end: its results and every total it announced.
async fn run(job: TransferJob) -> (TransferResults, Vec<usize>) {
    let (tx, mut rx) = EventSender::channel();
    let totals = tokio::spawn(async move {
        let mut totals = Vec::new();
        while let Some(event) = rx.recv().await {
            if let TransferEvent::ScanComplete { total_files, .. } = event {
                totals.push(total_files);
            }
        }
        totals
    });
    let results = run_job(job, tx).await.unwrap();
    (results, totals.await.unwrap())
}

fn sorted_names(archive: &Path) -> Vec<String> {
    let mut names = list_archive_files(archive).unwrap();
    names.sort();
    names
}

/// F6 of a folder into a zip with `*.txt` used to pack every file and
/// then delete them all from disk.
#[tokio::test]
async fn moving_into_a_zip_takes_and_deletes_only_what_the_filter_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, &[("keep.txt", b"keep")]);
    let docs = dir.path().join("docs");
    fs::create_dir_all(docs.join("sub")).unwrap();
    fs::write(docs.join("a.txt"), b"a").unwrap();
    fs::write(docs.join("b.log"), b"b").unwrap();
    fs::write(docs.join("sub").join("c.txt"), b"c").unwrap();

    let job = TransferJob::new(
        TransferOperation::Move,
        vec![docs.clone()],
        archive.clone(),
        filtered("*.txt"),
    );
    run(job).await;
    assert_eq!(
        sorted_names(&archive),
        [
            "docs/",
            "docs/a.txt",
            "docs/sub/",
            "docs/sub/c.txt",
            "keep.txt"
        ]
    );
    assert_eq!(fs::read(docs.join("b.log")).unwrap(), b"b");
    assert!(!docs.join("a.txt").exists());
    assert!(!docs.join("sub").exists());
}

/// Copying or moving out of an archive extracts only what the filter
/// keeps; a move leaves the rest in the archive.
#[tokio::test]
async fn extracting_takes_only_what_the_filter_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    let mut tree = SAMPLE_TREE.to_vec();
    tree.push(("sub/skip.log", b"log"));
    write_zip(&archive, &tree);
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();

    let copy = TransferJob::new(
        TransferOperation::Copy,
        vec![archive.join("sub")],
        out.clone(),
        filtered("*.txt"),
    );
    let (_, totals) = run(copy).await;
    let got: Vec<String> = snapshot(&out).into_keys().collect();
    assert_eq!(got, ["sub/b.txt"]);
    assert_eq!(totals, [1]);

    fs::remove_dir_all(out.join("sub")).unwrap();
    let mv = TransferJob::new(
        TransferOperation::Move,
        vec![archive.join("sub")],
        out.clone(),
        filtered("*.txt"),
    );
    run(mv).await;
    assert!(out.join("sub").join("b.txt").exists());
    assert_eq!(
        sorted_names(&archive),
        ["a.txt", "sub/deeper/c.bin", "sub/skip.log"]
    );
}

/// Extracting one file of a big archive announces one file, not the
/// archive's entry count; the result names where it came from and went.
#[tokio::test]
async fn extracting_a_selection_counts_and_reports_what_it_writes() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(
        &archive,
        &[
            ("f1.txt", b"1"),
            ("f2.txt", b"2"),
            ("f3.txt", b"3"),
            ("f4.txt", b"4"),
            ("f5.txt", b"5"),
        ],
    );
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();

    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![archive.join("f1.txt")],
        out.clone(),
        TransferOptions::default(),
    );
    let (results, totals) = run(job).await;
    assert_eq!(totals, [1]);
    let [done] = results.completed_files.as_slice() else {
        panic!("{:?}", results.completed_files);
    };
    assert_eq!(done.src, archive.join("f1.txt"));
    assert_eq!(done.dst, out.join("f1.txt"));
}

/// A file copied into an archive is reported from its local path to its
/// path inside the archive (the report used to land in the process's
/// working folder).
#[tokio::test]
async fn importing_reports_real_paths() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, &[("keep.txt", b"keep")]);
    let src = dir.path().join("x.txt");
    fs::write(&src, b"x").unwrap();

    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![src.clone()],
        archive.clone(),
        TransferOptions::default(),
    );
    let (results, _) = run(job).await;
    let [done] = results.completed_files.as_slice() else {
        panic!("{:?}", results.completed_files);
    };
    assert_eq!(done.src, src);
    assert_eq!(done.dst, archive.join("x.txt"));
}
