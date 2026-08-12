use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

use super::super::job::TransferOperation;
use super::super::options::TransferOptions;
use super::TransferWorker;

#[tokio::test]
async fn test_worker_move_directory_tree() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_root = temp_dir.path().join("src_folder");
    let dst_root = temp_dir.path().join("dst_folder");

    let sub_dir = src_root.join("sub_dir").join("nested");
    std::fs::create_dir_all(&sub_dir).unwrap();

    let file1 = src_root.join("file1.txt");
    let file2 = sub_dir.join("file2.txt");

    std::fs::write(&file1, "content1").unwrap();
    std::fs::write(&file2, "content2").unwrap();

    std::fs::create_dir_all(&dst_root).unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let is_paused = Arc::new(AtomicBool::new(false));
    let is_cancelled = Arc::new(AtomicBool::new(false));
    let skip_flag = Arc::new(AtomicBool::new(false));
    let active_conflict = Arc::new(std::sync::Mutex::new(None));

    let worker = TransferWorker::new(
        Uuid::new_v4(),
        TransferOperation::Move,
        vec![src_root.clone()],
        dst_root.clone(),
        TransferOptions::default(),
        is_paused,
        is_cancelled,
        skip_flag,
        tx,
        active_conflict,
    );

    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let res = worker.run().await;
    assert!(res.is_ok());

    // Verificar que el destino contenga todos los archivos y carpetas
    let dst_moved_folder = dst_root.join("src_folder");
    assert!(dst_moved_folder.join("file1.txt").exists());
    assert!(
        dst_moved_folder
            .join("sub_dir")
            .join("nested")
            .join("file2.txt")
            .exists()
    );

    // Verificar que el origen (archivos Y estructura de carpetas) fue eliminado por completo
    assert!(!file1.exists());
    assert!(!file2.exists());
    assert!(!sub_dir.exists());
    assert!(!src_root.exists());
}
