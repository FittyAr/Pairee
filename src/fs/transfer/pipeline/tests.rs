use super::*;
use crate::fs::transfer::job::{TransferJob, TransferOperation};

fn control() -> JobControl {
    let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let job = TransferJob::new(
        TransferOperation::Copy,
        vec![],
        PathBuf::new(),
        TransferOptions::default(),
    );
    JobControl::for_job(&job, tx)
}

/// Several aligned blocks plus an unaligned tail.
fn sample() -> Vec<u8> {
    (0..(3 * ALIGN + 100)).map(|i| (i % 251) as u8).collect()
}

async fn copy_with(options: TransferOptions) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.bin");
    let dst = dir.path().join("out").join("dst.bin");
    let data = sample();
    std::fs::write(&src, &data).unwrap();
    let acc = Arc::new(AtomicU64::new(0));
    let src_hash = copy_file_pipelined(&src, &dst, 0, &options, &control(), Arc::clone(&acc))
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), data);
    assert_eq!(acc.load(Ordering::SeqCst), data.len() as u64);
    assert_eq!(src_hash.is_some(), options.verify_after_copy);
}

#[tokio::test]
async fn copies_buffered_direct_and_sparse() {
    copy_with(TransferOptions::default()).await;
    copy_with(TransferOptions {
        verify_after_copy: true,
        direct_io: true,
        ..TransferOptions::default()
    })
    .await;
    copy_with(TransferOptions {
        produce_sparse_files: true,
        ..TransferOptions::default()
    })
    .await;
}

#[tokio::test]
async fn sparse_copy_keeps_zero_runs_and_length() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("zeros.bin");
    let dst = dir.path().join("zeros.out");
    let mut data = vec![0u8; 8 * ALIGN + 17];
    data[ALIGN * 2] = 1;
    std::fs::write(&src, &data).unwrap();
    let options = TransferOptions {
        produce_sparse_files: true,
        ..TransferOptions::default()
    };
    let acc = Arc::new(AtomicU64::new(0));
    copy_file_pipelined(&src, &dst, 0, &options, &control(), acc)
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), data);
}

#[tokio::test]
async fn resumed_copy_streams_only_the_rest_but_hashes_everything() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.bin");
    let part = dir.path().join("dst.bin.pairee-part");
    let data = sample();
    std::fs::write(&src, &data).unwrap();
    let kept = ALIGN + 7;
    std::fs::write(&part, &data[..kept]).unwrap();
    let options = TransferOptions {
        verify_after_copy: true,
        ..TransferOptions::default()
    };
    let acc = Arc::new(AtomicU64::new(0));
    let src_hash = copy_file_pipelined(&src, &part, kept as u64, &options, &control(), acc)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read(&part).unwrap(), data);
    let full = crate::fs::transfer::verify::hash_file_on_disk(
        &src,
        options.hash_algorithm,
        false,
        &|| false,
    )
    .unwrap();
    assert_eq!(src_hash, full);
}
