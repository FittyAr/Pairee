//! Copy out of, copy into and delete inside archives through the real
//! Transfer Engine backend dispatch.

use super::archive_vfs::{ArchivePlan, plan};
use super::run_job;
use crate::fs::archive::test_fixtures::{SAMPLE_TREE, snapshot, write_tar_gz, write_zip};
use crate::fs::archive::{ArchiveVfs, list_archive_files};
use crate::fs::transfer::conflict::ConflictResolution;
use crate::fs::transfer::events::{EventSender, TransferEvent};
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;
use crate::fs::vfs::Vfs;
use std::fs;
use std::path::{Path, PathBuf};

fn job(op: TransferOperation, sources: Vec<PathBuf>, destination: PathBuf) -> TransferJob {
    TransferJob::new(op, sources, destination, TransferOptions::default())
}

async fn run(job: TransferJob) -> anyhow::Result<()> {
    let (tx, mut rx) = EventSender::channel();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    run_job(job, tx).await.map(|_| ())
}

fn sorted_names(archive: &Path) -> Vec<String> {
    let mut names = list_archive_files(archive).unwrap();
    names.sort();
    names
}

#[test]
fn plans_follow_the_paths_of_the_job() {
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("a.zip");
    write_zip(&zip, SAMPLE_TREE);
    let out = dir.path().join("out");

    let local = job(
        TransferOperation::Copy,
        vec![dir.path().join("x")],
        out.clone(),
    );
    assert!(
        plan(&local).is_none(),
        "plain local copies keep their backend"
    );
    let whole = job(TransferOperation::Copy, vec![zip.clone()], out.clone());
    assert!(plan(&whole).is_none(), "copying the archive file itself");

    let extract = job(
        TransferOperation::Copy,
        vec![zip.join("sub").join("b.txt")],
        out,
    );
    assert_eq!(
        plan(&extract).unwrap().unwrap(),
        ArchivePlan::Extract {
            archive: zip.clone(),
            base: PathBuf::from("sub"),
            selected: vec![PathBuf::from("sub").join("b.txt")],
            dest: dir.path().join("out"),
        }
    );
    let moved = job(
        TransferOperation::Move,
        vec![zip.join("a.txt")],
        dir.path().into(),
    );
    assert!(
        plan(&moved).unwrap().is_err(),
        "moving out of an archive is refused"
    );
}

#[tokio::test]
async fn copy_out_of_an_archive_extracts_the_selection() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.tar.gz");
    write_tar_gz(&archive, SAMPLE_TREE);
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();

    let sources = vec![archive.join("sub")];
    run(job(TransferOperation::Copy, sources, out.clone()))
        .await
        .unwrap();
    let got: Vec<String> = snapshot(&out).into_keys().collect();
    assert_eq!(got, ["sub/b.txt", "sub/deeper/c.bin"]);
    assert_eq!(fs::read(out.join("sub").join("b.txt")).unwrap(), b"bravo");
}

#[tokio::test]
async fn copy_into_a_zip_and_delete_inside_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, &[("keep.txt", b"keep")]);
    let local = dir.path().join("docs");
    fs::create_dir_all(local.join("inner")).unwrap();
    fs::write(local.join("inner").join("n.txt"), b"note").unwrap();
    fs::write(dir.path().join("top.txt"), b"top").unwrap();

    let sources = vec![local, dir.path().join("top.txt")];
    run(job(TransferOperation::Copy, sources, archive.join("in")))
        .await
        .unwrap();
    assert_eq!(
        sorted_names(&archive),
        [
            "in/docs/",
            "in/docs/inner/",
            "in/docs/inner/n.txt",
            "in/top.txt",
            "keep.txt"
        ]
    );
    let vfs = ArchiveVfs::open(archive.clone()).unwrap();
    let note = archive.join("in").join("docs").join("inner").join("n.txt");
    assert_eq!(vfs.read_prefix(&note, 64).unwrap(), b"note");

    let doomed = vec![archive.join("in").join("docs")];
    run(job(TransferOperation::Delete, doomed, PathBuf::new()))
        .await
        .unwrap();
    assert_eq!(sorted_names(&archive), ["in/top.txt", "keep.txt"]);
}

#[tokio::test]
async fn writing_into_a_read_only_archive_fails_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.tar.gz");
    write_tar_gz(&archive, SAMPLE_TREE);
    let before = fs::read(&archive).unwrap();
    fs::write(dir.path().join("new.txt"), b"n").unwrap();

    let sources = vec![dir.path().join("new.txt")];
    assert!(
        run(job(TransferOperation::Copy, sources, archive.clone()))
            .await
            .is_err()
    );
    assert_eq!(fs::read(&archive).unwrap(), before, "archive untouched");
}

/// Copies a local `a.txt` ("new") into a zip already holding `a.txt`
/// ("old") with `mode`; "ask" is answered with `answer`.
async fn copy_over_existing(mode: &str, answer: ConflictResolution) -> (Vec<String>, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, &[("a.txt", b"old")]);
    let src = dir.path().join("a.txt");
    fs::write(&src, b"new").unwrap();
    let options = TransferOptions {
        conflict_resolution: mode.to_string(),
        ..TransferOptions::default()
    };
    let job = TransferJob::new(TransferOperation::Copy, vec![src], archive.clone(), options);
    let slot = std::sync::Arc::clone(&job.active_conflict);
    let (tx, mut rx) = EventSender::channel();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if matches!(event, TransferEvent::ConflictDetected { .. }) {
                slot.answer(answer);
            }
        }
    });
    run_job(job, tx).await.unwrap();
    let vfs = ArchiveVfs::open(archive.clone()).unwrap();
    let a = vfs.read_prefix(&archive.join("a.txt"), 64).unwrap();
    (sorted_names(&archive), a)
}

#[tokio::test]
async fn copies_into_a_zip_follow_the_conflict_setting() {
    let skip = copy_over_existing("skip", ConflictResolution::Skip).await;
    assert_eq!(skip, (vec!["a.txt".to_string()], b"old".to_vec()));
    let overwrite = copy_over_existing("overwrite", ConflictResolution::Skip).await;
    assert_eq!(overwrite, (vec!["a.txt".to_string()], b"new".to_vec()));
    let (names, a) = copy_over_existing("rename", ConflictResolution::Skip).await;
    assert_eq!(names, ["a (1).txt", "a.txt"]);
    assert_eq!(a, b"old");
}

#[tokio::test]
async fn asking_about_a_zip_conflict_waits_for_the_answer() {
    let asked_skip = copy_over_existing("ask", ConflictResolution::Skip).await;
    assert_eq!(asked_skip.1, b"old");
    let asked_overwrite = copy_over_existing("ask", ConflictResolution::Overwrite).await;
    assert_eq!(asked_overwrite.1, b"new");
}

/// The transfer dialog pre-fills `<folder>/<name>` for one item: that is
/// the target path, not a folder to create (regression: `b.txt/b.txt`).
#[tokio::test]
async fn single_item_target_path_is_not_made_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, SAMPLE_TREE);
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();

    let entry = archive.join("sub").join("b.txt");
    run(job(TransferOperation::Copy, vec![entry], out.join("b.txt")))
        .await
        .unwrap();
    assert_eq!(fs::read(out.join("b.txt")).unwrap(), b"bravo");

    let local = dir.path().join("new.txt");
    fs::write(&local, b"n").unwrap();
    let target = archive.join("sub").join("new.txt");
    run(job(TransferOperation::Copy, vec![local], target))
        .await
        .unwrap();
    assert!(sorted_names(&archive).contains(&"sub/new.txt".to_string()));
}
