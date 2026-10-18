//! The progress totals a local job announces match what it reports.

use super::job;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::events::{EventSender, TransferEvent};
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;

/// Every event `job` sends while it runs to the end.
async fn events_of(job: TransferJob) -> Vec<TransferEvent> {
    let (tx, mut rx) = EventSender::channel();
    let events = tokio::spawn(async move {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    });
    super::super::TransferWorker::for_job(job, tx)
        .run()
        .await
        .unwrap();
    events.await.unwrap()
}

/// Moves within one volume are renames: their files count in the total
/// (the bar used to read "3/0").
#[tokio::test]
async fn renamed_files_count_in_the_total() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let files: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|name| {
            let path = dir.path().join(name);
            std::fs::write(&path, name).unwrap();
            path
        })
        .collect();
    let sources: Vec<_> = files.iter().map(|p| p.as_path()).collect();
    let events = events_of(job(
        TransferOperation::Move,
        &sources,
        &out,
        TransferOptions::default(),
    ))
    .await;
    let totals: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            TransferEvent::ScanComplete { total_files, .. } => Some(*total_files),
            _ => None,
        })
        .collect();
    assert_eq!(totals, [3]);
    assert!(out.join("c").exists());
}

/// Low disk space is a warning in the log, not a skipped file.
#[test]
fn low_space_is_a_warning_not_a_skip() {
    let dir = tempfile::tempdir().unwrap();
    let job = TransferJob::new(
        TransferOperation::Copy,
        Vec::new(),
        dir.path().to_path_buf(),
        TransferOptions::default(),
    );
    let (tx, mut rx) = EventSender::channel();
    let ctl = JobControl::for_job(&job, tx);
    super::super::destination::warn_low_space(dir.path(), u64::MAX, &ctl);
    drop(ctl);
    let mut tracked = job;
    tracked.progress = Some(Default::default());
    let mut warned = false;
    while let Ok(event) = rx.try_recv() {
        warned |= matches!(event, TransferEvent::Warning { .. });
        crate::fs::transfer::job_log::apply_event(&mut tracked, &event);
    }
    assert!(warned);
    assert_eq!(tracked.progress.unwrap().files_skipped, 0);
    assert!(tracked.results.skipped_files.is_empty());
}
