//! Grouping the plan into Transfer Engine jobs, and running them.

use super::*;
use crate::fs::sync::plan::PlannedJob;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;

#[test]
fn copies_are_grouped_by_folder_and_deletes_by_side() {
    let (l, r) = trees();
    put(l.path(), "a.txt", "a", 0);
    put(l.path(), "b.txt", "b", 0);
    put(l.path(), "sub/c.txt", "c", 0);
    put(r.path(), "sub/keep.txt", "k", 0);
    put(r.path(), "x.txt", "x", 0);
    put(r.path(), "y.txt", "y", 0);
    let items = diff(l.path(), r.path(), &opts(SyncDirection::Mirror));

    let jobs = plan_jobs(&items);
    assert_eq!(
        jobs,
        [
            PlannedJob {
                operation: TransferOperation::Copy,
                sources: vec![l.path().join("a.txt"), l.path().join("b.txt")],
                destination: r.path().to_path_buf(),
            },
            PlannedJob {
                operation: TransferOperation::Copy,
                sources: vec![l.path().join("sub").join("c.txt")],
                destination: r.path().join("sub").join("c.txt"),
            },
            PlannedJob {
                operation: TransferOperation::Delete,
                sources: vec![
                    r.path().join("sub").join("keep.txt"),
                    r.path().join("x.txt"),
                    r.path().join("y.txt"),
                ],
                destination: PathBuf::new(),
            },
        ]
    );
}

#[test]
fn transfer_jobs_overwrite_keep_times_and_carry_the_mask() {
    let job = PlannedJob {
        operation: TransferOperation::Copy,
        sources: vec![PathBuf::from("a")],
        destination: PathBuf::from("b"),
    };
    let base = TransferOptions {
        preserve_timestamps: false,
        delete_to_recycle_bin: true,
        ..TransferOptions::default()
    };
    let transfer = job.clone().into_transfer_job(&base, "*.rs;!target");
    assert_eq!(transfer.options.conflict_resolution, "overwrite");
    assert!(transfer.options.preserve_timestamps);
    assert!(transfer.options.delete_to_recycle_bin, "base options kept");
    assert_eq!(
        transfer.options.filter_mask.as_deref(),
        Some("*.rs;!target")
    );
    assert_eq!(transfer.sources, job.sources);
    assert_eq!(job.into_transfer_job(&base, "").options.filter_mask, None);
}

/// Runs the plan on the real local Transfer Engine backend.
async fn apply(items: &[SyncItem], mask: &str) {
    for planned in plan_jobs(items) {
        let job = planned.into_transfer_job(&TransferOptions::default(), mask);
        let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        crate::fs::transfer::backend::run_job(job, tx)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn applying_a_mirror_plan_makes_both_trees_equal() {
    let (l, r) = trees();
    put(l.path(), "changed.txt", "left version", 600);
    put(r.path(), "changed.txt", "right", 0);
    put(l.path(), "new/deep/file.txt", "deep", 0);
    put(l.path(), "new/target/skip.o", "obj", 0);
    put(l.path(), "top.txt", "1", 0);
    put(l.path(), "pending.txt", "p", 0);
    put(r.path(), "pending.txt", "p", 0);
    put(r.path(), "extra/old.txt", "old", 0);
    put(r.path(), "extra.txt", "old", 0);

    let mut options = opts(SyncDirection::Mirror);
    options.mask = "!target".into();
    let items = diff(l.path(), r.path(), &options);
    apply(&items, &options.effective_mask()).await;

    let after = diff(l.path(), r.path(), &options);
    assert!(
        after.iter().all(|i| i.kind == DiffKind::Equal),
        "{after:#?}"
    );
    assert_eq!(
        std::fs::read_to_string(r.path().join("changed.txt")).unwrap(),
        "left version"
    );
    assert!(r.path().join("new/deep/file.txt").exists());
    assert!(
        !r.path().join("new/target").exists(),
        "excluded folder is not copied"
    );
    assert!(!r.path().join("extra").exists());
    assert!(!r.path().join("extra.txt").exists());
}

#[tokio::test]
async fn applying_both_directions_exchanges_new_files() {
    let (l, r) = trees();
    put(l.path(), "from_left.txt", "L", 0);
    put(r.path(), "from_right/inner.txt", "R", 0);
    put(l.path(), "shared.txt", "newer on left", 900);
    put(r.path(), "shared.txt", "older", 0);

    let options = opts(SyncDirection::Both);
    apply(&diff(l.path(), r.path(), &options), "").await;

    assert!(
        diff(l.path(), r.path(), &options)
            .iter()
            .all(|i| i.kind == DiffKind::Equal)
    );
    assert!(l.path().join("from_right/inner.txt").exists());
    assert!(r.path().join("from_left.txt").exists());
}
