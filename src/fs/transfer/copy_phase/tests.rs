//! The stream path (used for SFTP) exercised on local folders: the same
//! adapters' primitives (`create_file`, `replace`, `set_metadata`) without a
//! server.

use super::*;
use crate::fs::transfer::job::TransferJob;
use crate::fs::transfer::part::part_path;
use crate::fs::transfer::scan::{ScanRequest, scan, top_targets};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Local folders reached through the stream copy, as an SSH job would.
fn stream_endpoints() -> Endpoints {
    Endpoints {
        local: false,
        ..Endpoints::local()
    }
}

async fn run_stream(
    operation: TransferOperation,
    sources: &[PathBuf],
    destination: &Path,
    options: TransferOptions,
) -> TransferResults {
    let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let job = TransferJob::new(
        operation,
        sources.to_vec(),
        destination.to_path_buf(),
        options,
    );
    let ctl = JobControl::for_job(&job, tx);
    let endpoints = stream_endpoints();
    let roots = top_targets(sources, destination, operation, endpoints.dst.as_ref());
    let request = ScanRequest {
        roots: &roots,
        operation,
        options: &job.options,
        src_fs: endpoints.src.as_ref(),
        dst_fs: endpoints.dst.as_ref(),
        same_fs: true,
        dst_links: false,
    };
    let scan = scan(&request, &ctl).unwrap();
    let request = CopyRequest {
        operation,
        scan,
        options: &job.options,
        endpoints,
        slot: job.active_conflict.clone(),
    };
    run_copy_phase(request, &ctl, TransferResults::default())
        .await
        .unwrap()
}

/// FS-09: SSH copies honour the conflict setting, verify, keep times and
/// replace atomically.
#[tokio::test]
async fn stream_copies_resolve_conflicts_verify_and_keep_times() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("new.txt"), b"new").unwrap();
    std::fs::write(src.join("sub").join("old.txt"), b"source").unwrap();
    let when = SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000);
    filetime::set_file_mtime(
        src.join("new.txt"),
        filetime::FileTime::from_system_time(when),
    )
    .unwrap();
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(dst.join("src").join("sub")).unwrap();
    std::fs::write(dst.join("src").join("sub").join("old.txt"), b"keep").unwrap();

    let options = TransferOptions {
        conflict_resolution: "skip".into(),
        verify_after_copy: true,
        ..TransferOptions::default()
    };
    let results = run_stream(
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &dst,
        options,
    )
    .await;
    assert_eq!(results.completed_files.len(), 1);
    assert!(results.completed_files[0].verified);
    assert_eq!(results.skipped_files.len(), 1);
    let copied = dst.join("src").join("new.txt");
    assert_eq!(std::fs::read(&copied).unwrap(), b"new");
    assert_eq!(
        std::fs::metadata(&copied).unwrap().modified().unwrap(),
        when
    );
    assert!(!part_path(&copied).exists());
    assert_eq!(
        std::fs::read(dst.join("src").join("sub").join("old.txt")).unwrap(),
        b"keep"
    );
}

/// FS-09 / FS-12: a stream move deletes each source only after its copy,
/// and the emptied folders afterwards.
#[tokio::test]
async fn stream_moves_remove_sources_after_copying() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("tree");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("sub").join("f.txt"), b"f").unwrap();
    let dst = dir.path().join("out");
    std::fs::create_dir(&dst).unwrap();
    let results = run_stream(
        TransferOperation::Move,
        std::slice::from_ref(&src),
        &dst,
        TransferOptions::default(),
    )
    .await;
    assert_eq!(results.completed_files.len(), 1);
    assert!(!src.exists());
    assert_eq!(
        std::fs::read(dst.join("tree").join("sub").join("f.txt")).unwrap(),
        b"f"
    );
}
